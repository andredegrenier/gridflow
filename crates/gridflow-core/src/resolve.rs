//! Evaluates the AST into a `DiagramModel`: binds variables, expands class
//! instantiations, interpolates labels, validates ports, creates phantom nodes
//! for edges that reference undeclared ids.
//!
//! Resolution degrades per statement like the parser does: an unknown `$var`
//! or bad argument produces a diagnostic and the statement renders with
//! defaults — the canvas never goes blank because one line is broken.

use crate::ast::*;
use crate::library::LibraryProvider;
use crate::model::*;
use crate::parser::parse;
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(Rgba),
    Num(f32),
    Str(String),
    Bundle(Vec<ResolvedAttr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedAttr {
    Shape(Shape),
    Dashed,
    Bold,
    Fill(Rgba),
    Stroke(Rgba),
    TextColor(Rgba),
    Width(f32),
    Height(f32),
    /// Already resolved to a glyph (names went through the icon table).
    Icon(SmolStr),
}

#[derive(Clone)]
enum Binding {
    Value(Value),
    Class(ClassStmt),
    /// Name is taken by a node instantiation (prevents reuse as a variable).
    NodeInstance,
}

pub fn resolve(ast: &Ast, lib: &dyn LibraryProvider) -> DiagramModel {
    let mut r = Resolver {
        lib,
        local: HashMap::new(),
        imported: HashMap::new(),
        model: DiagramModel::default(),
        pending_edges: Vec::new(),
        node_defaults: Vec::new(),
        edge_defaults: Vec::new(),
    };
    r.model.diagnostics = ast.diagnostics.clone();
    for stmt in &ast.stmts {
        r.stmt(stmt, None);
    }
    r.finish_edges();
    r.model
}

struct Resolver<'a> {
    lib: &'a dyn LibraryProvider,
    local: HashMap<SmolStr, Binding>,
    /// name -> (binding, which library file it came from)
    imported: HashMap<SmolStr, (Binding, SmolStr)>,
    model: DiagramModel,
    pending_edges: Vec<PendingEdge>,
    /// Baselines from `default node [...]` / `default edge [...]`, applied to
    /// every node/edge declared after the statement.
    node_defaults: Vec<ResolvedAttr>,
    edge_defaults: Vec<ResolvedAttr>,
}

/// One chain segment, captured with the defaults in force at its statement.
struct PendingEdge {
    from: EndpointRef,
    to: EndpointRef,
    arrow: ArrowKind,
    label: Option<StrLit>,
    attrs: Vec<AttrItem>,
    defaults: Vec<ResolvedAttr>,
    stmt_span: Span,
}

impl<'a> Resolver<'a> {
    fn diag(&mut self, span: Span, msg: impl Into<String>) {
        self.model.diagnostics.push(Diagnostic::error(span, msg));
    }
    fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.model.diagnostics.push(Diagnostic::warning(span, msg));
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.local
            .get(name)
            .or_else(|| self.imported.get(name).map(|(b, _)| b))
    }

    // ---- statements ------------------------------------------------------

    fn stmt(&mut self, stmt: &Stmt, group: Option<&NodeId>) {
        match stmt {
            Stmt::Use(u) => self.use_stmt(u),
            Stmt::Dir(d) => self.model.direction = d.dir,
            Stmt::Default(d) => {
                let mut diags = Vec::new();
                let items = eval_attr_items(&d.attrs, &|n| self.lookup(n), &mut diags);
                self.model.diagnostics.extend(diags);
                match d.target {
                    DefaultTarget::Node => self.node_defaults = items,
                    DefaultTarget::Edge => self.edge_defaults = items,
                }
            }
            Stmt::Class(c) => self.bind(
                c.name.clone(),
                Binding::Class(c.clone()),
                c.name_span,
            ),
            Stmt::Assign(a) => self.assign(a, group),
            Stmt::Node(n) => self.node_stmt(n, group),
            Stmt::Edge(e) => {
                // Expand chains here; endpoints resolve in the second pass so
                // forward references still work.
                for (i, arrow) in e.arrows.iter().enumerate() {
                    let (mut from, mut to) = (e.endpoints[i].clone(), e.endpoints[i + 1].clone());
                    if arrow.reversed {
                        std::mem::swap(&mut from, &mut to);
                    }
                    self.pending_edges.push(PendingEdge {
                        from,
                        to,
                        arrow: arrow.kind,
                        label: e.label.clone(),
                        attrs: e.attrs.clone(),
                        defaults: self.edge_defaults.clone(),
                        stmt_span: e.stmt_span,
                    });
                }
            }
            Stmt::Group(g) => self.group_stmt(g),
        }
    }

    fn bind(&mut self, name: SmolStr, binding: Binding, span: Span) {
        match self.local.entry(name) {
            std::collections::hash_map::Entry::Occupied(e) => {
                let name = e.key().clone();
                self.diag(span, format!("`{name}` is already defined (single assignment)"));
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(binding);
            }
        }
    }

    fn use_stmt(&mut self, u: &UseStmt) {
        for (name, span) in &u.names {
            let Some(src) = self.lib.load(name) else {
                self.diag(*span, format!("library `{name}` not found"));
                continue;
            };
            let ast = parse(&src);
            // Evaluate the library file's assignments in its own scope so its
            // classes can rely on its variables; then import the results.
            let mut lib_local: HashMap<SmolStr, Binding> = HashMap::new();
            for stmt in &ast.stmts {
                match stmt {
                    Stmt::Assign(a) => {
                        if let AssignRhs::Value(expr) = &a.rhs {
                            let val = eval_value(expr, &|n| lib_local.get(n), &mut Vec::new());
                            if let Some(v) = val {
                                lib_local.entry(a.name.clone()).or_insert(Binding::Value(v));
                            }
                        }
                    }
                    Stmt::Class(c) => {
                        lib_local
                            .entry(c.name.clone())
                            .or_insert(Binding::Class(c.clone()));
                    }
                    _ => {}
                }
            }
            for (bname, binding) in lib_local {
                match self.imported.get(&bname) {
                    Some((_, prev_lib)) if prev_lib != name => {
                        let prev = prev_lib.clone();
                        self.diag(
                            *span,
                            format!("`{bname}` is defined by both `{prev}` and `{name}`; qualify by removing one `use`"),
                        );
                    }
                    Some(_) => {}
                    None => {
                        self.imported.insert(bname, (binding, name.clone()));
                    }
                }
            }
        }
    }

    fn assign(&mut self, a: &AssignStmt, group: Option<&NodeId>) {
        match &a.rhs {
            AssignRhs::Value(expr) => {
                if let Some(p) = &a.placement {
                    let span = p.span();
                    self.warn(span, "placement on a value assignment has no effect");
                }
                let mut diags = Vec::new();
                let val = eval_value(expr, &|n| self.lookup(n), &mut diags);
                self.model.diagnostics.extend(diags);
                if let Some(v) = val {
                    self.bind(a.name.clone(), Binding::Value(v), a.name_span);
                }
            }
            AssignRhs::Call { class, class_span, args, .. } => {
                self.instantiate(a, class, *class_span, args, group);
            }
        }
    }

    fn instantiate(
        &mut self,
        a: &AssignStmt,
        class_name: &SmolStr,
        class_span: Span,
        args: &[ValueExpr],
        group: Option<&NodeId>,
    ) {
        let class = match self.lookup(class_name) {
            Some(Binding::Class(c)) => c.clone(),
            Some(_) => {
                self.diag(class_span, format!("`{class_name}` is not a class"));
                return;
            }
            None => {
                self.diag(class_span, format!("unknown class `{class_name}`"));
                return;
            }
        };

        if args.len() > class.params.len() {
            self.diag(
                class_span,
                format!(
                    "`{class_name}` takes {} argument(s), got {}",
                    class.params.len(),
                    args.len()
                ),
            );
        }

        // Bind parameters: positional args evaluated in the caller's scope,
        // then defaults, then diagnostics for anything still missing.
        let mut env: HashMap<SmolStr, Binding> = HashMap::new();
        let mut diags = Vec::new();
        for (i, param) in class.params.iter().enumerate() {
            let value = match args.get(i) {
                Some(expr) => eval_value(expr, &|n| self.lookup(n), &mut diags),
                None => match &param.default {
                    Some(expr) => eval_value(expr, &|n| self.lookup(n), &mut diags),
                    None => {
                        diags.push(Diagnostic::error(
                            class_span,
                            format!("missing argument `{}` for `{class_name}`", param.name),
                        ));
                        None
                    }
                },
            };
            if let Some(v) = value {
                env.insert(param.name.clone(), Binding::Value(v));
            }
        }

        // Evaluate class variables in order; each sees params and earlier vars,
        // falling back to the caller's scope for anything else.
        for (vname, expr) in &class.vars {
            let val = eval_value(
                expr,
                &|n| env.get(n).or_else(|| self.lookup(n)),
                &mut diags,
            );
            if let Some(v) = val {
                env.insert(vname.clone(), Binding::Value(v));
            }
        }
        self.model.diagnostics.extend(diags);

        // Map well-known variables onto the node.
        let mut shape = Shape::Rect;
        let mut style = NodeStyle::default();
        let defaults = self.node_defaults.clone();
        apply_attrs(&mut shape, &mut style, &defaults);
        let mut label = None;
        let get = |env: &HashMap<SmolStr, Binding>, name: &str| -> Option<Value> {
            match env.get(name) {
                Some(Binding::Value(v)) => Some(v.clone()),
                _ => None,
            }
        };
        if let Some(Value::Bundle(items)) = get(&env, "shape") {
            apply_attrs(&mut shape, &mut style, &items);
        }
        match get(&env, "label") {
            Some(Value::Str(s)) => label = Some(s),
            Some(Value::Num(n)) => label = Some(trim_float(n)),
            Some(_) => self.diag(class_span, "`label` must be a string"),
            None => {}
        }
        if let Some(v) = get(&env, "fill") {
            match v {
                Value::Color(c) => style.fill = Some(c),
                _ => self.diag(class_span, "`fill` must be a color"),
            }
        }
        if let Some(v) = get(&env, "stroke") {
            match v {
                Value::Color(c) => style.stroke = Some(c),
                _ => self.diag(class_span, "`stroke` must be a color"),
            }
        }
        if let Some(Value::Num(n)) = get(&env, "w") {
            style.width = Some(n);
        }
        if let Some(Value::Num(n)) = get(&env, "h") {
            style.height = Some(n);
        }
        if let Some(v) = get(&env, "icon") {
            match v {
                Value::Str(s) => style.icon = Some(SmolStr::new(crate::icons::glyph(&s))),
                _ => self.diag(class_span, "`icon` must be a string"),
            }
        }

        let ports = class
            .ports
            .iter()
            .map(|p| Port { name: p.name.clone(), side: p.side })
            .collect();

        let node = Node {
            id: a.name.clone(),
            label,
            shape,
            style,
            ports,
            placement: placement_from_ast(a.placement.as_ref()),
            group: group.cloned(),
            stmt_span: a.stmt_span,
            insert_placement_at: a.insert_placement_at,
            phantom: false,
        };
        self.insert_node(node, a.name_span);
        self.bind_instance_name(a.name.clone());
    }

    fn bind_instance_name(&mut self, name: SmolStr) {
        self.local.entry(name).or_insert(Binding::NodeInstance);
    }

    fn node_stmt(&mut self, n: &NodeStmt, group: Option<&NodeId>) {
        let mut shape = Shape::Rect;
        let mut style = NodeStyle::default();
        let defaults = self.node_defaults.clone();
        apply_attrs(&mut shape, &mut style, &defaults);

        if n.applied_vars.len() > 1 {
            for (_, span) in &n.applied_vars[1..] {
                self.diag(
                    *span,
                    "only one `$var` may be applied per node (compose bundles at definition instead)",
                );
            }
        }
        if let Some((vname, vspan)) = n.applied_vars.first() {
            match self.lookup(vname).cloned() {
                Some(Binding::Value(Value::Bundle(items))) => {
                    apply_attrs(&mut shape, &mut style, &items)
                }
                Some(Binding::Value(_)) => {
                    self.diag(*vspan, format!("`${vname}` must be an attribute bundle to apply to a node"))
                }
                Some(_) => self.diag(*vspan, format!("`${vname}` is not a variable")),
                None => self.diag(*vspan, format!("unknown variable `${vname}`")),
            }
        }

        // Inline attrs override anything the bundle set.
        let mut diags = Vec::new();
        let items = eval_attr_items(&n.attrs, &|name| self.lookup(name), &mut diags);
        self.model.diagnostics.extend(diags);
        apply_attrs(&mut shape, &mut style, &items);

        let label = n.label.as_ref().map(|l| {
            let mut diags = Vec::new();
            let s = interpolate(l, &|name| self.lookup(name), &mut diags);
            self.model.diagnostics.extend(diags);
            s
        });

        let node = Node {
            id: n.id.clone(),
            label,
            shape,
            style,
            ports: Vec::new(),
            placement: placement_from_ast(n.placement.as_ref()),
            group: group.cloned(),
            stmt_span: n.stmt_span,
            insert_placement_at: n.insert_placement_at,
            phantom: false,
        };
        self.insert_node(node, n.id_span);
        self.bind_instance_name(n.id.clone());
    }

    fn insert_node(&mut self, node: Node, id_span: Span) {
        if let Some(existing) = self.model.nodes.get_mut(&node.id) {
            if existing.phantom {
                // A phantom created by an earlier edge: the real declaration wins,
                // regardless of statement order.
                *existing = node;
            } else {
                self.diag(id_span, format!("node `{}` is already declared", node.id));
            }
        } else {
            let id = node.id.clone();
            self.model.nodes.insert(id, node);
        }
    }

    fn group_stmt(&mut self, g: &GroupStmt) {
        let label = g.label.as_ref().map(|l| {
            let mut diags = Vec::new();
            let s = interpolate(l, &|name| self.lookup(name), &mut diags);
            self.model.diagnostics.extend(diags);
            s
        });
        if self.model.groups.contains_key(&g.id) {
            self.diag(g.id_span, format!("group `{}` is already declared", g.id));
            return;
        }
        let gid = g.id.clone();
        for stmt in &g.body {
            self.stmt(stmt, Some(&gid));
        }
        let members = self
            .model
            .nodes
            .values()
            .filter(|n| n.group.as_ref() == Some(&gid))
            .map(|n| n.id.clone())
            .collect();
        self.model.groups.insert(
            gid.clone(),
            Group {
                id: gid,
                label,
                placement: placement_from_ast(g.placement.as_ref()),
                dir: g.dir,
                members,
                stmt_span: g.stmt_span,
                insert_placement_at: g.insert_placement_at,
            },
        );
    }

    // ---- edges (second pass: forward references allowed) ----------------

    fn finish_edges(&mut self) {
        let edges = std::mem::take(&mut self.pending_edges);
        for e in edges {
            self.ensure_endpoint(&e.from);
            self.ensure_endpoint(&e.to);

            let mut dashed = e.arrow == ArrowKind::Dotted;
            let mut bold = false;
            let mut stroke = None;
            let mut diags = Vec::new();
            let items = eval_attr_items(&e.attrs, &|name| self.lookup(name), &mut diags);
            self.model.diagnostics.extend(diags);
            for item in e.defaults.iter().cloned().chain(items) {
                match item {
                    ResolvedAttr::Dashed => dashed = true,
                    ResolvedAttr::Bold => bold = true,
                    ResolvedAttr::Stroke(c) | ResolvedAttr::Fill(c) => stroke = Some(c),
                    _ => {}
                }
            }

            let label = e.label.as_ref().map(|l| {
                let mut diags = Vec::new();
                let s = interpolate(l, &|name| self.lookup(name), &mut diags);
                self.model.diagnostics.extend(diags);
                s
            });

            self.model.edges.push(Edge {
                from: e.from.node.clone(),
                from_port: e.from.port.clone(),
                to: e.to.node.clone(),
                to_port: e.to.port.clone(),
                arrow: e.arrow,
                label,
                dashed,
                bold,
                stroke,
                stmt_span: e.stmt_span,
            });
        }
    }

    fn ensure_endpoint(&mut self, ep: &EndpointRef) {
        if let Some(node) = self.model.nodes.get(&ep.node) {
            if let Some(port) = &ep.port {
                if !node.ports.iter().any(|p| &p.name == port) {
                    self.diag(
                        ep.span,
                        format!("node `{}` has no port `{port}`", ep.node),
                    );
                }
            }
            return;
        }
        // Phantom: referenced but never declared. Renders dashed; a later real
        // declaration in this same parse would have already filled nodes, and
        // insert_node upgrades phantoms if declarations come after edges.
        self.model.nodes.insert(
            ep.node.clone(),
            Node {
                id: ep.node.clone(),
                label: None,
                shape: Shape::Rect,
                style: NodeStyle { dashed: true, ..Default::default() },
                ports: Vec::new(),
                placement: Placement::Auto,
                group: None,
                stmt_span: ep.span,
                insert_placement_at: ep.span.end,
                phantom: true,
            },
        );
    }
}

// ---- shared evaluation helpers ------------------------------------------

fn placement_from_ast(p: Option<&PlacementAst>) -> Placement {
    match p {
        None => Placement::Auto,
        Some(PlacementAst::Absolute { x, y, span }) => {
            Placement::Absolute { pos: [*x, *y], span: *span }
        }
        Some(PlacementAst::Relative { anchor, dir, gap, span, .. }) => Placement::Relative {
            anchor: anchor.clone(),
            dir: *dir,
            gap: *gap,
            span: *span,
        },
    }
}

fn trim_float(n: f32) -> String {
    if n.fract() == 0.0 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn value_to_display(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Num(n) => trim_float(*n),
        Value::Color(Rgba([r, g, b, a])) => {
            if *a == 255 {
                format!("#{r:02x}{g:02x}{b:02x}")
            } else {
                format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
            }
        }
        Value::Bundle(_) => "[bundle]".to_string(),
    }
}

fn interpolate<'e>(
    lit: &StrLit,
    lookup: &dyn Fn(&str) -> Option<&'e Binding>,
    diags: &mut Vec<Diagnostic>,
) -> String {
    let mut out = String::new();
    for seg in &lit.segments {
        match seg {
            StrSeg::Lit(s) => out.push_str(s),
            StrSeg::Var(name) => match lookup(name) {
                Some(Binding::Value(v)) => out.push_str(&value_to_display(v)),
                _ => {
                    diags.push(Diagnostic::error(
                        lit.span,
                        format!("unknown variable `${name}` in string"),
                    ));
                    out.push('$');
                    out.push_str(name);
                }
            },
        }
    }
    out
}

fn eval_value<'e>(
    expr: &ValueExpr,
    lookup: &dyn Fn(&str) -> Option<&'e Binding>,
    diags: &mut Vec<Diagnostic>,
) -> Option<Value> {
    match expr {
        ValueExpr::Color { rgba, .. } => Some(Value::Color(Rgba(*rgba))),
        ValueExpr::Num { value, .. } => Some(Value::Num(*value)),
        ValueExpr::Str(lit) => Some(Value::Str(interpolate(lit, lookup, diags))),
        ValueExpr::VarRef { name, span } => match lookup(name) {
            Some(Binding::Value(v)) => Some(v.clone()),
            Some(_) => {
                diags.push(Diagnostic::error(*span, format!("`${name}` is not a variable")));
                None
            }
            None => {
                diags.push(Diagnostic::error(*span, format!("unknown variable `${name}`")));
                None
            }
        },
        ValueExpr::Bundle { items, .. } => {
            Some(Value::Bundle(eval_attr_items(items, lookup, diags)))
        }
    }
}

fn eval_attr_items<'e>(
    items: &[AttrItem],
    lookup: &dyn Fn(&str) -> Option<&'e Binding>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<ResolvedAttr> {
    let mut out = Vec::new();
    for item in items {
        match item {
            AttrItem::Shape(s) => out.push(ResolvedAttr::Shape(*s)),
            AttrItem::Dashed => out.push(ResolvedAttr::Dashed),
            AttrItem::Bold => out.push(ResolvedAttr::Bold),
            AttrItem::Fill(e) => match eval_value(e, lookup, diags) {
                Some(Value::Color(c)) => out.push(ResolvedAttr::Fill(c)),
                Some(_) => diags.push(Diagnostic::error(e.span(), "`fill` must be a color")),
                None => {}
            },
            AttrItem::Stroke(e) => match eval_value(e, lookup, diags) {
                Some(Value::Color(c)) => out.push(ResolvedAttr::Stroke(c)),
                Some(_) => diags.push(Diagnostic::error(e.span(), "`stroke` must be a color")),
                None => {}
            },
            AttrItem::TextColor(e) => match eval_value(e, lookup, diags) {
                Some(Value::Color(c)) => out.push(ResolvedAttr::TextColor(c)),
                Some(_) => diags.push(Diagnostic::error(e.span(), "`text` must be a color")),
                None => {}
            },
            AttrItem::Width(e) => match eval_value(e, lookup, diags) {
                Some(Value::Num(n)) => out.push(ResolvedAttr::Width(n)),
                Some(_) => diags.push(Diagnostic::error(e.span(), "`w` must be a number")),
                None => {}
            },
            AttrItem::Height(e) => match eval_value(e, lookup, diags) {
                Some(Value::Num(n)) => out.push(ResolvedAttr::Height(n)),
                Some(_) => diags.push(Diagnostic::error(e.span(), "`h` must be a number")),
                None => {}
            },
            AttrItem::Icon(e) => match eval_value(e, lookup, diags) {
                Some(Value::Str(s)) => {
                    if !crate::icons::is_known(&s)
                        && s.len() > 1
                        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        diags.push(Diagnostic::warning(
                            e.span(),
                            format!("unknown icon name `{s}` (rendering it as literal text)"),
                        ));
                    }
                    out.push(ResolvedAttr::Icon(SmolStr::new(crate::icons::glyph(&s))));
                }
                Some(_) => diags.push(Diagnostic::error(
                    e.span(),
                    "`icon` must be a name or a string glyph",
                )),
                None => {}
            },
            AttrItem::Splice { name, span } => match lookup(name) {
                Some(Binding::Value(Value::Bundle(items))) => out.extend(items.iter().cloned()),
                Some(Binding::Value(_)) => diags.push(Diagnostic::error(
                    *span,
                    format!("`${name}` must be a bundle to splice into `[...]`"),
                )),
                Some(_) => {
                    diags.push(Diagnostic::error(*span, format!("`${name}` is not a variable")))
                }
                None => diags.push(Diagnostic::error(*span, format!("unknown variable `${name}`"))),
            },
        }
    }
    out
}

/// Apply resolved attrs in order — later items overwrite earlier ones.
fn apply_attrs(shape: &mut Shape, style: &mut NodeStyle, items: &[ResolvedAttr]) {
    for item in items {
        match item {
            ResolvedAttr::Shape(s) => *shape = *s,
            ResolvedAttr::Dashed => style.dashed = true,
            ResolvedAttr::Bold => style.bold = true,
            ResolvedAttr::Fill(c) => style.fill = Some(*c),
            ResolvedAttr::Stroke(c) => style.stroke = Some(*c),
            ResolvedAttr::TextColor(c) => style.text = Some(*c),
            ResolvedAttr::Width(n) => style.width = Some(*n),
            ResolvedAttr::Height(n) => style.height = Some(*n),
            ResolvedAttr::Icon(g) => style.icon = Some(g.clone()),
        }
    }
}
