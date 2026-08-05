//! Mermaid flowchart support: parse mermaid source into the same
//! `DiagramModel` that GFD produces, so mermaid documents get the full
//! gridflow experience — crisp pan/zoom canvas, layered layout, themes,
//! SVG/PNG export — without translation.
//!
//! Scope: `flowchart`/`graph` diagrams (nodes, all bracket shapes, edge
//! variants with labels, `&` fan-in/out, subgraphs with `direction`,
//! `classDef`/`class`/`:::`/`style`, `%%` comments). Other mermaid diagram
//! types produce a clear diagnostic instead of a broken parse.
//!
//! Like the GFD parser, recovery is per statement: one bad line never blanks
//! the canvas. Spans index into the mermaid source so editor underlines and
//! double-click-to-source work identically.

use crate::ast::{parse_hex_color, ArrowKind, Diagnostic, LayoutDir, Shape, Span};
use crate::model::*;
use smol_str::SmolStr;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Detection

/// Does this text look like mermaid rather than GFD? Used by `Document` to
/// pick the parser; writing either language in the editor Just Works.
pub fn is_mermaid(src: &str) -> bool {
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.starts_with("%%") {
            return true; // mermaid comment or %%{init}%% directive
        }
        let first = t.split_whitespace().next().unwrap_or("");
        let rest = t[first.len()..].trim();
        return match first {
            "flowchart" | "flowchart-elk" => true,
            // `graph` is a legal GFD node id; only treat it as a mermaid
            // header when what follows is a direction (or nothing).
            "graph" => rest.is_empty() || LayoutDir::from_keyword(rest).is_some(),
            _ => OTHER_DIAGRAM_TYPES.contains(&first),
        };
    }
    false
}

const OTHER_DIAGRAM_TYPES: &[&str] = &[
    "sequenceDiagram",
    "classDiagram",
    "stateDiagram",
    "stateDiagram-v2",
    "erDiagram",
    "gantt",
    "pie",
    "journey",
    "mindmap",
    "timeline",
    "quadrantChart",
    "gitGraph",
    "sankey-beta",
    "xychart-beta",
    "block-beta",
    "requirementDiagram",
    "C4Context",
];

// ---------------------------------------------------------------------------
// Parsing

pub fn parse(src: &str) -> DiagramModel {
    let mut p = MermaidParser {
        src,
        model: DiagramModel::default(),
        class_defs: HashMap::new(),
        class_uses: Vec::new(),
        style_uses: Vec::new(),
        current_group: None,
        group_stack: Vec::new(),
        saw_header: false,
        unsupported: false,
        anon_group: 0,
    };
    p.run();
    p.finalize();
    p.model
}

#[derive(Debug, Clone, Default)]
struct StyleProps {
    fill: Option<Rgba>,
    stroke: Option<Rgba>,
    text: Option<Rgba>,
    dashed: bool,
    bold: bool,
}

struct MermaidParser<'a> {
    src: &'a str,
    model: DiagramModel,
    class_defs: HashMap<String, StyleProps>,
    /// (node id, class name, span) — applied in finalize (order-independent).
    class_uses: Vec<(NodeId, String, Span)>,
    style_uses: Vec<(NodeId, StyleProps, Span)>,
    current_group: Option<NodeId>,
    group_stack: Vec<NodeId>,
    saw_header: bool,
    /// Set when the header names a non-flowchart diagram type; the rest of
    /// the file is skipped so one clear error isn't buried in parse noise.
    unsupported: bool,
    anon_group: u32,
}

impl<'a> MermaidParser<'a> {
    fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.model.diagnostics.push(Diagnostic::warning(span, msg));
    }
    fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.model.diagnostics.push(Diagnostic::error(span, msg));
    }

    fn run(&mut self) {
        for (line, line_start) in lines_with_offsets(self.src) {
            // Statements can also be separated by `;` outside brackets/quotes.
            for (stmt, stmt_start) in split_semis(line, line_start) {
                let trimmed_start = stmt_start + (stmt.len() - stmt.trim_start().len());
                let stmt = stmt.trim();
                if stmt.is_empty() || stmt.starts_with("%%") {
                    continue;
                }
                self.statement(stmt, trimmed_start);
            }
        }
        if !self.group_stack.is_empty() || self.current_group.is_some() {
            let end = Span::new(self.src.len(), self.src.len());
            self.warn(end, "subgraph without matching `end`");
        }
    }

    fn statement(&mut self, stmt: &str, offset: usize) {
        if self.unsupported {
            return;
        }
        let span_all = Span::new(offset, offset + stmt.len());
        let first = stmt.split_whitespace().next().unwrap_or("");
        let rest = stmt[first.len()..].trim_start();
        let rest_offset = offset + stmt.len() - stmt[first.len()..].trim_start().len();

        match first {
            "flowchart" | "flowchart-elk" | "graph" if !self.saw_header => {
                self.saw_header = true;
                if !rest.is_empty() {
                    match LayoutDir::from_keyword(rest) {
                        Some(dir) => self.model.direction = dir,
                        None => self.warn(
                            Span::new(rest_offset, rest_offset + rest.len()),
                            format!("unknown flow direction `{rest}` (expected TB, TD, BT, LR or RL)"),
                        ),
                    }
                }
            }
            _ if !self.saw_header && OTHER_DIAGRAM_TYPES.contains(&first) => {
                self.saw_header = true;
                self.unsupported = true;
                self.error(
                    span_all,
                    format!(
                        "gridflow renders mermaid *flowcharts*; `{first}` diagrams aren't supported yet"
                    ),
                );
            }
            "subgraph" => self.subgraph(rest, rest_offset, span_all),
            "end" if rest.is_empty() => {
                if self.current_group.is_some() {
                    self.current_group = self.group_stack.pop();
                } else {
                    self.warn(span_all, "`end` without an open subgraph");
                }
            }
            "direction" => match LayoutDir::from_keyword(rest) {
                Some(dir) => {
                    if let Some(gid) = self.current_group.clone() {
                        if let Some(g) = self.model.groups.get_mut(&gid) {
                            g.dir = Some(dir);
                        }
                    } else {
                        self.model.direction = dir;
                    }
                }
                None => self.warn(span_all, format!("unknown direction `{rest}`")),
            },
            "classDef" => self.class_def(rest, rest_offset),
            "class" => self.class_stmt(rest, rest_offset),
            "style" => self.style_stmt(rest, rest_offset),
            // Interactivity / styling statements that don't affect geometry.
            "linkStyle" | "click" | "accTitle:" | "accDescr" | "accDescr:" | "title" => {}
            _ => self.node_edge_statement(stmt, offset),
        }
    }

    // -- subgraph -----------------------------------------------------------

    fn subgraph(&mut self, rest: &str, offset: usize, span_all: Span) {
        if let Some(cur) = self.current_group.clone() {
            self.warn(
                span_all,
                "nested subgraphs are flattened (gridflow groups don't nest)",
            );
            self.group_stack.push(cur);
        }
        let (id, label) = if rest.is_empty() {
            self.anon_group += 1;
            (SmolStr::new(format!("subgraph_{}", self.anon_group)), None)
        } else {
            let mut cur = Cursor::new(rest, offset);
            let id_start = cur.pos;
            let ident = cur.ident();
            cur.skip_ws();
            if !ident.is_empty() && cur.peek() == Some(b'[') {
                // `subgraph id [Title]`
                cur.bump();
                let mut title = cur.until_close(b']').unwrap_or_default();
                title = unquote(&title);
                (SmolStr::new(ident), Some(clean_text(&title)))
            } else if !ident.is_empty() && cur.at_end() {
                (SmolStr::new(ident), Some(ident.to_string()))
            } else {
                // `subgraph free text title`
                let _ = id_start;
                (SmolStr::new(sanitize_id(rest)), Some(rest.to_string()))
            }
        };
        if self.model.groups.contains_key(&id) {
            self.warn(span_all, format!("subgraph `{id}` is already declared"));
            self.current_group = Some(id);
            return;
        }
        self.model.groups.insert(
            id.clone(),
            Group {
                id: id.clone(),
                label,
                placement: Placement::Auto,
                dir: None,
                members: Vec::new(),
                stmt_span: span_all,
                insert_placement_at: span_all.end,
            },
        );
        self.current_group = Some(id);
    }

    // -- classDef / class / style ------------------------------------------

    fn class_def(&mut self, rest: &str, offset: usize) {
        let Some((names, props)) = rest.split_once(char::is_whitespace) else {
            self.warn(
                Span::new(offset, offset + rest.len()),
                "classDef needs a name and properties",
            );
            return;
        };
        let style = parse_style_props(props);
        for name in names.split(',') {
            self.class_defs.insert(name.trim().to_string(), style.clone());
        }
    }

    fn class_stmt(&mut self, rest: &str, offset: usize) {
        let span = Span::new(offset, offset + rest.len());
        let Some((ids, name)) = rest.rsplit_once(char::is_whitespace) else {
            self.warn(span, "class statement needs node ids and a class name");
            return;
        };
        let name = name.trim().to_string();
        for id in ids.split(',') {
            let id = id.trim();
            if !id.is_empty() {
                self.class_uses.push((SmolStr::new(id), name.clone(), span));
            }
        }
    }

    fn style_stmt(&mut self, rest: &str, offset: usize) {
        let span = Span::new(offset, offset + rest.len());
        let Some((id, props)) = rest.split_once(char::is_whitespace) else {
            self.warn(span, "style statement needs a node id and properties");
            return;
        };
        self.style_uses
            .push((SmolStr::new(id.trim()), parse_style_props(props), span));
    }

    // -- node / edge statements --------------------------------------------

    fn node_edge_statement(&mut self, stmt: &str, offset: usize) {
        let mut cur = Cursor::new(stmt, offset);
        let Some(mut lhs) = self.node_group(&mut cur) else {
            self.error(
                Span::new(offset, offset + stmt.len()),
                "expected a node or edge statement",
            );
            return;
        };
        loop {
            cur.skip_ws();
            if cur.at_end() {
                break;
            }
            let Some(arrow) = self.arrow(&mut cur) else {
                let here = cur.pos;
                self.error(
                    Span::new(here, offset + stmt.len()),
                    format!("expected an arrow, found `{}`", cur.remainder_preview()),
                );
                return;
            };
            cur.skip_ws();
            let Some(rhs) = self.node_group(&mut cur) else {
                self.error(
                    Span::new(cur.pos, offset + stmt.len()),
                    "expected a node after the arrow",
                );
                return;
            };
            let stmt_span = Span::new(offset, cur.pos);
            for from in &lhs {
                for to in &rhs {
                    let (mut a, mut b) = (from.clone(), to.clone());
                    if arrow.reversed {
                        std::mem::swap(&mut a, &mut b);
                    }
                    self.model.edges.push(Edge {
                        from: a,
                        from_port: None,
                        to: b,
                        to_port: None,
                        arrow: arrow.kind,
                        label: arrow.label.clone(),
                        dashed: arrow.dashed,
                        bold: arrow.bold,
                        stroke: None,
                        stmt_span,
                    });
                }
            }
            lhs = rhs;
        }
    }

    /// `a & b & c` — returns the node ids, declaring nodes as encountered.
    fn node_group(&mut self, cur: &mut Cursor) -> Option<Vec<NodeId>> {
        let mut ids = vec![self.node(cur)?];
        loop {
            cur.skip_ws();
            if cur.peek() == Some(b'&') {
                cur.bump();
                cur.skip_ws();
                ids.push(self.node(cur)?);
            } else {
                return Some(ids);
            }
        }
    }

    /// One node reference: `id`, `id[Text]`, `id((Text))`, `>Text]`, …
    /// optionally followed by `:::className`.
    fn node(&mut self, cur: &mut Cursor) -> Option<NodeId> {
        cur.skip_ws();
        let start = cur.pos;
        let ident = cur.ident();
        if ident.is_empty() {
            return None;
        }
        let id = SmolStr::new(ident);
        let mut shape_label: Option<(Shape, String)> = None;

        // Bracket shapes. Longest opener first.
        const FORMS: &[(&str, &str, Shape)] = &[
            ("(((", ")))", Shape::DblCircle),
            ("((", "))", Shape::Circle),
            ("([", "])", Shape::Stadium),
            ("[[", "]]", Shape::Subroutine),
            ("[(", ")]", Shape::Cylinder),
            ("{{", "}}", Shape::Hexagon),
            ("(", ")", Shape::Rounded),
            ("{", "}", Shape::Diamond),
            (">", "]", Shape::Tag),
        ];
        if cur.starts_with("[/") || cur.starts_with("[\\") {
            // Slanted quads: closing delimiter decides the shape.
            let open_fwd = cur.starts_with("[/");
            cur.advance(2);
            let (text, closer) = cur.until_either("/]", "\\]")?;
            let close_fwd = closer == "/]";
            let shape = if open_fwd == close_fwd { Shape::Parallelogram } else { Shape::Trapezoid };
            shape_label = Some((shape, text));
        } else if cur.peek() == Some(b'[') && !cur.starts_with("[[") && !cur.starts_with("[(") {
            cur.bump();
            shape_label = Some((Shape::Rect, cur.until_close(b']')?));
        } else {
            for (open, close, shape) in FORMS {
                if cur.starts_with(open) {
                    cur.advance(open.len());
                    shape_label = Some((*shape, cur.bracket_text(close)?));
                    break;
                }
            }
        }

        // `:::className`
        let mut class_name = None;
        if cur.starts_with(":::") {
            cur.advance(3);
            let name = cur.ident();
            if !name.is_empty() {
                class_name = Some(name.to_string());
            }
        }
        let span = Span::new(start, cur.pos);
        if let Some(name) = class_name {
            self.class_uses.push((id.clone(), name, span));
        }

        self.ensure_node(&id, shape_label, span);
        Some(id)
    }

    fn ensure_node(&mut self, id: &NodeId, shape_label: Option<(Shape, String)>, span: Span) {
        let group = self.current_group.clone();
        if let Some(node) = self.model.nodes.get_mut(id) {
            // Later mention with brackets upgrades a bare reference.
            if let Some((shape, text)) = shape_label {
                node.shape = shape;
                node.label = Some(clean_text(&unquote(&text)));
                node.stmt_span = span;
            }
            return;
        }
        let (shape, label) = match shape_label {
            Some((shape, text)) => (shape, Some(clean_text(&unquote(&text)))),
            None => (Shape::Rect, None),
        };
        self.model.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                label,
                shape,
                style: NodeStyle::default(),
                ports: Vec::new(),
                placement: Placement::Auto,
                group: group.clone(),
                stmt_span: span,
                insert_placement_at: span.end,
                phantom: false,
            },
        );
        if let Some(gid) = group {
            if let Some(g) = self.model.groups.get_mut(&gid) {
                g.members.push(id.clone());
            }
        }
    }

    /// Arrow token, including inline (`-- label -->`) and pipe (`-->|label|`)
    /// label forms.
    fn arrow(&mut self, cur: &mut Cursor) -> Option<ParsedArrow> {
        cur.skip_ws();
        let start = cur.pos;
        let token = cur.take_while(|c| matches!(c, b'-' | b'=' | b'.' | b'<' | b'>'));
        if token.is_empty() {
            return None;
        }
        // Minimum arrow is two chars (`--`, `==`, `-.`, `<-`).
        if token.len() < 2 || !token.contains(['-', '=', '.']) {
            cur.pos = start;
            return None;
        }
        let mut label = None;

        // Open inline-label forms: exactly `--` / `-.` / `==` with text next.
        let complete = token.ends_with('>') || token.len() >= 3;
        let mut token = token.to_string();
        if !complete {
            let closers: &[&str] = match token.as_str() {
                "--" => &["-->", "---"],
                "-." => &[".->", ".-"],
                "==" => &["==>", "==="],
                "<-" | "<=" => &[], // half of <--/<==; fall through
                _ => &[],
            };
            if !closers.is_empty() {
                let (text, closer) = cur.until_either(closers[0], closers[1])?;
                label = Some(clean_text(&unquote(text.trim())));
                token.push_str(closer);
            }
        }

        let kind;
        let mut reversed = false;
        let heads_right = token.ends_with('>');
        let heads_left = token.starts_with('<');
        let dotted = token.contains('.');
        let bold = token.contains('=');
        if heads_right && heads_left {
            kind = ArrowKind::Bidirectional;
        } else if heads_right {
            kind = if dotted { ArrowKind::Dotted } else { ArrowKind::Directed };
        } else if heads_left {
            kind = if dotted { ArrowKind::Dotted } else { ArrowKind::Directed };
            reversed = true;
        } else {
            kind = ArrowKind::Undirected;
        }

        // `|label|` after the arrow.
        cur.skip_ws();
        if cur.peek() == Some(b'|') {
            cur.bump();
            let text = cur.until_close(b'|')?;
            label = Some(clean_text(&unquote(text.trim())));
        }

        Some(ParsedArrow { kind, reversed, dashed: dotted, bold, label })
    }

    // -- finalize -----------------------------------------------------------

    fn finalize(&mut self) {
        // Undeclared edge endpoints still need nodes (mermaid implicitly
        // declares on first reference — our parser already does — but a
        // `class`/`style` on an unknown id should warn).
        let default_def = self.class_defs.get("default").cloned();
        if let Some(def) = default_def {
            for node in self.model.nodes.values_mut() {
                apply_props(&mut node.style, &def);
            }
        }
        let uses = std::mem::take(&mut self.class_uses);
        for (id, name, span) in uses {
            let Some(def) = self.class_defs.get(&name).cloned() else {
                self.warn(span, format!("unknown class `{name}`"));
                continue;
            };
            match self.model.nodes.get_mut(&id) {
                Some(node) => apply_props(&mut node.style, &def),
                None => self.warn(span, format!("class applied to unknown node `{id}`")),
            }
        }
        let styles = std::mem::take(&mut self.style_uses);
        for (id, props, span) in styles {
            match self.model.nodes.get_mut(&id) {
                Some(node) => apply_props(&mut node.style, &props),
                None => self.warn(span, format!("style applied to unknown node `{id}`")),
            }
        }
    }
}

struct ParsedArrow {
    kind: ArrowKind,
    reversed: bool,
    dashed: bool,
    bold: bool,
    label: Option<String>,
}

fn apply_props(style: &mut NodeStyle, props: &StyleProps) {
    if let Some(c) = props.fill {
        style.fill = Some(c);
    }
    if let Some(c) = props.stroke {
        style.stroke = Some(c);
    }
    if let Some(c) = props.text {
        style.text = Some(c);
    }
    style.dashed |= props.dashed;
    style.bold |= props.bold;
}

fn parse_style_props(props: &str) -> StyleProps {
    let mut out = StyleProps::default();
    for pair in props.split(',') {
        let Some((k, v)) = pair.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "fill" => out.fill = parse_css_color(v),
            "stroke" => out.stroke = parse_css_color(v),
            "color" => out.text = parse_css_color(v),
            "stroke-dasharray" => out.dashed = true,
            "stroke-width" => {
                let n: f32 = v.trim_end_matches("px").parse().unwrap_or(1.0);
                if n >= 2.5 {
                    out.bold = true;
                }
            }
            "font-weight" if v == "bold" => out.bold = true,
            _ => {}
        }
    }
    out
}

fn parse_css_color(v: &str) -> Option<Rgba> {
    if v.starts_with('#') {
        return parse_hex_color(v).map(Rgba);
    }
    let named: &[(&str, [u8; 3])] = &[
        ("black", [0x00, 0x00, 0x00]),
        ("white", [0xff, 0xff, 0xff]),
        ("red", [0xe5, 0x39, 0x35]),
        ("green", [0x43, 0xa0, 0x47]),
        ("blue", [0x1e, 0x88, 0xe5]),
        ("yellow", [0xfd, 0xd8, 0x35]),
        ("orange", [0xfb, 0x8c, 0x00]),
        ("purple", [0x8e, 0x24, 0xaa]),
        ("pink", [0xec, 0x40, 0x7a]),
        ("gray", [0x9e, 0x9e, 0x9e]),
        ("grey", [0x9e, 0x9e, 0x9e]),
        ("lightblue", [0xad, 0xd8, 0xe6]),
        ("lightgreen", [0x90, 0xee, 0x90]),
        ("lightyellow", [0xff, 0xff, 0xe0]),
        ("lightgray", [0xd3, 0xd3, 0xd3]),
        ("lightgrey", [0xd3, 0xd3, 0xd3]),
    ];
    named
        .iter()
        .find(|(n, _)| *n == v)
        .map(|(_, [r, g, b])| Rgba::rgb(*r, *g, *b))
}

/// `<br>`-family tags become newlines; each resulting line is trimmed.
fn clean_text(s: &str) -> String {
    let mut out = s.to_string();
    for tag in ["<br/>", "<br />", "<br>"] {
        out = out.replace(tag, "\n");
    }
    out.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('`') && t.ends_with('`'))) {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

/// Mermaid titles used as ids: keep word chars, everything else becomes `_`.
fn sanitize_id(s: &str) -> String {
    let mut out: String = s
        .trim()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

// ---------------------------------------------------------------------------
// Low-level scanning

/// Lines with their byte offsets into the full source.
fn lines_with_offsets(src: &str) -> impl Iterator<Item = (&str, usize)> {
    let mut offset = 0usize;
    src.split('\n').map(move |line| {
        let this = offset;
        offset += line.len() + 1;
        (line, this)
    })
}

/// Split a line on `;` at bracket depth 0 and outside quotes.
fn split_semis(line: &str, base: usize) -> Vec<(&str, usize)> {
    let bytes = line.as_bytes();
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => in_quote = !in_quote,
            b'[' | b'(' | b'{' if !in_quote => depth += 1,
            b']' | b')' | b'}' if !in_quote => depth -= 1,
            b';' if !in_quote && depth <= 0 => {
                parts.push((&line[start..i], base + start));
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push((&line[start..], base + start));
    parts
}

/// Byte cursor over one statement. All delimiters are ASCII, so byte indexing
/// always slices on UTF-8 boundaries.
struct Cursor<'a> {
    s: &'a str,
    /// Absolute byte position (offset by the statement's base), so spans made
    /// from `pos` index the full document.
    pos: usize,
    base: usize,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a str, base: usize) -> Self {
        Cursor { s, pos: base, base }
    }

    fn rel(&self) -> usize {
        self.pos - self.base
    }
    fn at_end(&self) -> bool {
        self.rel() >= self.s.len()
    }
    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.rel()).copied()
    }
    fn bump(&mut self) {
        self.pos += 1;
    }
    fn advance(&mut self, n: usize) {
        self.pos += n;
    }
    fn rest(&self) -> &'a str {
        &self.s[self.rel().min(self.s.len())..]
    }
    fn starts_with(&self, pat: &str) -> bool {
        self.rest().starts_with(pat)
    }
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ') | Some(b'\t') | Some(b'\r')) {
            self.bump();
        }
    }

    fn remainder_preview(&self) -> String {
        self.rest().chars().take(12).collect()
    }

    /// Identifier: word chars plus interior `-`/`.` (mermaid allows both,
    /// but `--` always starts an arrow).
    fn ident(&mut self) -> &'a str {
        let start = self.rel();
        let bytes = self.s.as_bytes();
        let mut i = start;
        while i < bytes.len() {
            let b = bytes[i];
            let word = b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80;
            if word {
                i += 1;
            } else if (b == b'-' || b == b'.') && i > start {
                // Interior only, and never when it would eat into an arrow.
                match bytes.get(i + 1) {
                    Some(&n) if (n.is_ascii_alphanumeric() || n == b'_' || n >= 0x80) => i += 2,
                    _ => break,
                }
            } else {
                break;
            }
        }
        let out = &self.s[start..i];
        self.pos = self.base + i;
        out
    }

    fn take_while(&mut self, f: impl Fn(u8) -> bool) -> &'a str {
        let start = self.rel();
        let bytes = self.s.as_bytes();
        let mut i = start;
        while i < bytes.len() && f(bytes[i]) {
            i += 1;
        }
        let out = &self.s[start..i];
        self.pos = self.base + i;
        out
    }

    /// Text until a single-byte closer, honoring `"…"` protection.
    fn until_close(&mut self, close: u8) -> Option<String> {
        let bytes = self.s.as_bytes();
        let mut i = self.rel();
        let start = i;
        let mut in_quote = false;
        while i < bytes.len() {
            let b = bytes[i];
            if b == b'"' {
                in_quote = !in_quote;
            } else if b == close && !in_quote {
                let out = self.s[start..i].to_string();
                self.pos = self.base + i + 1;
                return Some(out);
            }
            i += 1;
        }
        None
    }

    /// Text until a multi-byte closing string.
    fn until_str(&mut self, close: &str) -> Option<String> {
        let rest = self.rest();
        let at = rest.find(close)?;
        let out = rest[..at].to_string();
        self.pos += at + close.len();
        Some(out)
    }

    /// Bracket body: a leading `"…"` protects the closer (`a("text (x)")`);
    /// otherwise everything up to the closing string.
    fn bracket_text(&mut self, close: &str) -> Option<String> {
        if self.peek() == Some(b'"') {
            self.bump();
            let text = self.until_close(b'"')?;
            self.skip_ws();
            if self.starts_with(close) {
                self.advance(close.len());
                return Some(text);
            }
            return None;
        }
        self.until_str(close)
    }

    /// Text until whichever of two closers appears first; returns which one.
    fn until_either(&mut self, a: &'static str, b: &'static str) -> Option<(String, &'static str)> {
        let rest = self.rest();
        let fa = rest.find(a);
        let fb = rest.find(b);
        let (at, closer) = match (fa, fb) {
            (Some(x), Some(y)) if x <= y => (x, a),
            (Some(x), None) => (x, a),
            (_, Some(y)) => (y, b),
            (None, None) => return None,
        };
        let out = rest[..at].to_string();
        self.pos += at + closer.len();
        Some((out, closer))
    }
}

// ---------------------------------------------------------------------------
// Mermaid -> GFD conversion

/// Generate equivalent GFD source from a (mermaid-parsed) model. Used by the
/// app's "Convert to GFD" command so mermaid users can graduate to two-way
/// editing (drag-to-pin needs GFD's placement syntax).
pub fn to_gfd(model: &DiagramModel) -> String {
    // Mermaid ids may contain `-`/`.` which GFD identifiers don't allow.
    // Statement-leading GFD keywords can't be node ids.
    const GFD_KEYWORDS: &[&str] = &["use", "class", "group", "node", "default", "dir"];
    let mut rename: HashMap<&NodeId, String> = HashMap::new();
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    for id in model.nodes.keys().chain(model.groups.keys()) {
        let mut clean = sanitize_id(id);
        if GFD_KEYWORDS.contains(&clean.as_str()) {
            clean.push('_');
        }
        while !taken.insert(clean.clone()) {
            clean.push('_');
        }
        rename.insert(id, clean);
    }
    let name = |id: &NodeId| -> String {
        rename.get(id).cloned().unwrap_or_else(|| sanitize_id(id))
    };

    let mut out = String::new();
    out.push_str("// Converted from mermaid by gridflow\n");
    if model.direction != LayoutDir::default() {
        out.push_str(&format!("dir: {}\n", model.direction.keyword()));
    }
    out.push('\n');

    let node_line = |node: &Node| -> String {
        let mut line = name(&node.id);
        let label = node.label.as_deref().unwrap_or("");
        if !label.is_empty() && label != node.id.as_str() {
            line.push_str(&format!(" \"{}\"", escape_label(label)));
        }
        let mut attrs: Vec<String> = Vec::new();
        if node.shape != Shape::Rect {
            attrs.push(node.shape.keyword().to_string());
        }
        if let Some(c) = node.style.fill {
            attrs.push(format!("fill={}", hex(c)));
        }
        if let Some(c) = node.style.stroke {
            attrs.push(format!("stroke={}", hex(c)));
        }
        if let Some(c) = node.style.text {
            attrs.push(format!("text={}", hex(c)));
        }
        if let Some(g) = &node.style.icon {
            attrs.push(format!("icon=\"{g}\""));
        }
        if node.style.dashed {
            attrs.push("dashed".into());
        }
        if node.style.bold {
            attrs.push("bold".into());
        }
        if !attrs.is_empty() {
            line.push_str(&format!(" [{}]", attrs.join(", ")));
        }
        line
    };

    for group in model.groups.values() {
        let gname = name(&group.id);
        match &group.label {
            Some(l) if !l.is_empty() => {
                out.push_str(&format!("group {gname} \"{}\" {{\n", escape_label(l)))
            }
            _ => out.push_str(&format!("group {gname} {{\n")),
        }
        if let Some(dir) = group.dir {
            out.push_str(&format!("  dir: {}\n", dir.keyword()));
        }
        for m in &group.members {
            if let Some(node) = model.nodes.get(m) {
                out.push_str(&format!("  {}\n", node_line(node)));
            }
        }
        out.push_str("}\n\n");
    }

    let grouped: std::collections::HashSet<&NodeId> = model
        .groups
        .values()
        .flat_map(|g| g.members.iter())
        .collect();
    for node in model.nodes.values() {
        if !grouped.contains(&node.id) {
            out.push_str(&node_line(node));
            out.push('\n');
        }
    }
    if model.nodes.values().any(|n| !grouped.contains(&n.id)) {
        out.push('\n');
    }

    for e in &model.edges {
        let arrow = match e.arrow {
            ArrowKind::Directed => "->",
            ArrowKind::Bidirectional => "<->",
            ArrowKind::Undirected => "--",
            ArrowKind::Dotted => "..>",
        };
        let mut line = format!("{} {} {}", name(&e.from), arrow, name(&e.to));
        if let Some(l) = &e.label {
            line.push_str(&format!(" : \"{}\"", escape_label(l)));
        }
        let mut attrs: Vec<String> = Vec::new();
        if e.dashed && e.arrow != ArrowKind::Dotted {
            attrs.push("dashed".into());
        }
        if e.bold {
            attrs.push("bold".into());
        }
        if let Some(c) = e.stroke {
            attrs.push(format!("stroke={}", hex(c)));
        }
        if !attrs.is_empty() {
            line.push_str(&format!(" [{}]", attrs.join(", ")));
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn hex(Rgba([r, g, b, a]): Rgba) -> String {
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn model(src: &str) -> DiagramModel {
        parse(src)
    }

    fn errors(m: &DiagramModel) -> Vec<String> {
        m.diagnostics
            .iter()
            .filter(|d| d.severity == crate::ast::Severity::Error)
            .map(|d| d.message.clone())
            .collect()
    }

    #[test]
    fn detects_mermaid_vs_gfd() {
        assert!(is_mermaid("flowchart TD\na-->b"));
        assert!(is_mermaid("graph LR\na-->b"));
        assert!(is_mermaid("%% comment\ngraph TD"));
        assert!(is_mermaid("sequenceDiagram\nA->>B: hi"));
        assert!(!is_mermaid("node a \"Hello\""));
        assert!(!is_mermaid("graph \"A node named graph\""));
        assert!(!is_mermaid("// comment\na -> b"));
    }

    #[test]
    fn parses_basic_flowchart() {
        let m = model("flowchart LR\n  a[Start] --> b(Round) --> c{Choice}\n");
        assert_eq!(m.direction, LayoutDir::LeftRight);
        assert_eq!(m.nodes.len(), 3);
        assert_eq!(m.nodes["a"].label.as_deref(), Some("Start"));
        assert_eq!(m.nodes["a"].shape, Shape::Rect);
        assert_eq!(m.nodes["b"].shape, Shape::Rounded);
        assert_eq!(m.nodes["c"].shape, Shape::Diamond);
        assert_eq!(m.edges.len(), 2);
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
    }

    #[test]
    fn all_bracket_shapes_map() {
        let src = "graph TD
  a([stadium])
  b[[subroutine]]
  c[(db)]
  d((circle))
  e(((dbl)))
  f{{hex}}
  g[/para/]
  h[\\para2\\]
  i[/trap\\]
  j[\\trapinv/]
  k>tag]
";
        let m = model(src);
        assert_eq!(m.nodes["a"].shape, Shape::Stadium);
        assert_eq!(m.nodes["b"].shape, Shape::Subroutine);
        assert_eq!(m.nodes["c"].shape, Shape::Cylinder);
        assert_eq!(m.nodes["d"].shape, Shape::Circle);
        assert_eq!(m.nodes["e"].shape, Shape::DblCircle);
        assert_eq!(m.nodes["f"].shape, Shape::Hexagon);
        assert_eq!(m.nodes["g"].shape, Shape::Parallelogram);
        assert_eq!(m.nodes["h"].shape, Shape::Parallelogram);
        assert_eq!(m.nodes["i"].shape, Shape::Trapezoid);
        assert_eq!(m.nodes["j"].shape, Shape::Trapezoid);
        assert_eq!(m.nodes["k"].shape, Shape::Tag);
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
    }

    #[test]
    fn edge_variants_and_labels() {
        let m = model(
            "graph TD
  a --> b
  a --- c
  a -.-> d
  a -.- e
  a ==> f
  a <--> g
  a -->|to h| h
  a -- inline --> i
  a -. dotted label .-> j
  a == thick label ==> k
",
        );
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
        let by_to = |to: &str| m.edges.iter().find(|e| e.to == to).unwrap();
        assert_eq!(by_to("b").arrow, ArrowKind::Directed);
        assert_eq!(by_to("c").arrow, ArrowKind::Undirected);
        assert_eq!(by_to("d").arrow, ArrowKind::Dotted);
        assert!(by_to("d").dashed);
        assert_eq!(by_to("e").arrow, ArrowKind::Undirected);
        assert!(by_to("e").dashed);
        assert!(by_to("f").bold);
        assert_eq!(by_to("g").arrow, ArrowKind::Bidirectional);
        assert_eq!(by_to("h").label.as_deref(), Some("to h"));
        assert_eq!(by_to("i").label.as_deref(), Some("inline"));
        assert_eq!(by_to("j").label.as_deref(), Some("dotted label"));
        assert!(by_to("j").dashed);
        assert_eq!(by_to("k").label.as_deref(), Some("thick label"));
        assert!(by_to("k").bold);
    }

    #[test]
    fn fan_in_out_with_ampersand() {
        let m = model("graph LR\n  a & b --> c & d\n");
        assert_eq!(m.edges.len(), 4);
        assert!(errors(&m).is_empty());
    }

    #[test]
    fn chains_declare_and_upgrade() {
        let m = model("graph TD\n  a --> b\n  b[Real Label]\n");
        assert_eq!(m.nodes["b"].label.as_deref(), Some("Real Label"));
        assert!(!m.nodes["b"].phantom);
    }

    #[test]
    fn subgraphs_become_groups() {
        let m = model(
            "flowchart TB
subgraph backend [Back end]
  direction LR
  api --> db[(store)]
end
web --> api
",
        );
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
        let g = &m.groups["backend"];
        assert_eq!(g.label.as_deref(), Some("Back end"));
        assert_eq!(g.dir, Some(LayoutDir::LeftRight));
        assert_eq!(g.members, vec![SmolStr::new("api"), SmolStr::new("db")]);
        assert_eq!(m.nodes["web"].group, None);
        assert_eq!(m.nodes["api"].group.as_deref(), Some("backend"));
    }

    #[test]
    fn class_defs_and_styles_apply() {
        let m = model(
            "graph TD
  a[One]:::hot
  b[Two]
  c[Three]
  classDef hot fill:#f96,stroke:#333
  class b hot
  style c fill:#00ff00,stroke-width:4px
",
        );
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
        assert_eq!(m.nodes["a"].style.fill, Some(Rgba([0xff, 0x99, 0x66, 255])));
        assert_eq!(m.nodes["b"].style.fill, Some(Rgba([0xff, 0x99, 0x66, 255])));
        assert_eq!(m.nodes["c"].style.fill, Some(Rgba([0x00, 0xff, 0x00, 255])));
        assert!(m.nodes["c"].style.bold);
    }

    #[test]
    fn quoted_text_and_br() {
        let m = model("graph TD\n  a[\"has [brackets] & <br> break\"]\n");
        assert_eq!(
            m.nodes["a"].label.as_deref(),
            Some("has [brackets] &\nbreak")
        );
    }

    #[test]
    fn unsupported_diagram_type_gives_one_clear_error() {
        let m = model("sequenceDiagram\n  A->>B: hi\n");
        let errs = errors(&m);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("flowchart"));
    }

    #[test]
    fn bad_line_recovers() {
        let m = model("graph TD\n  a --> b\n  ??!!\n  b --> c\n");
        assert_eq!(m.edges.len(), 2);
        assert!(!errors(&m).is_empty());
    }

    #[test]
    fn semicolon_statements() {
        let m = model("graph LR; a-->b; b-->c;");
        assert_eq!(m.edges.len(), 2);
        assert!(errors(&m).is_empty(), "{:?}", errors(&m));
    }

    #[test]
    fn spans_index_into_source() {
        let src = "graph TD\n  alpha[Hi] --> beta\n";
        let m = model(src);
        let a = &m.nodes["alpha"];
        assert_eq!(&src[a.stmt_span.start..a.stmt_span.end], "alpha[Hi]");
        assert_eq!(m.node_at_byte(src.find("alpha").unwrap() + 1), Some(&SmolStr::new("alpha")));
    }

    #[test]
    fn dashed_ids_parse() {
        let m = model("graph TD\n  my-node --> other.node\n");
        assert!(m.nodes.contains_key("my-node"));
        assert!(m.nodes.contains_key("other.node"));
        assert_eq!(m.edges.len(), 1);
    }

    #[test]
    fn to_gfd_roundtrips_through_gfd_parser() {
        let m = model(
            "flowchart LR
subgraph svc [Services]
  api(API) --> q([queue])
end
web[Web<br>frontend] -->|https| api
q -.-> worker-1[[Worker]]
classDef hot fill:#f96
class api hot
",
        );
        let gfd = to_gfd(&m);
        let reparsed = crate::resolve::resolve(
            &crate::parser::parse(&gfd),
            &crate::library::NoLibrary,
        );
        let errs: Vec<_> = reparsed
            .diagnostics
            .iter()
            .filter(|d| d.severity == crate::ast::Severity::Error)
            .collect();
        assert!(errs.is_empty(), "generated GFD has errors: {errs:?}\n---\n{gfd}");
        assert_eq!(reparsed.nodes.len(), m.nodes.len());
        assert_eq!(reparsed.edges.len(), m.edges.len());
        assert_eq!(reparsed.groups.len(), 1);
        // Styles survive the trip.
        let api = reparsed.nodes.get("api").unwrap();
        assert_eq!(api.style.fill, Some(Rgba([0xff, 0x99, 0x66, 255])));
        // Multi-line label survives via \n escape.
        assert!(reparsed
            .nodes
            .values()
            .any(|n| n.label.as_deref() == Some("Web\nfrontend")));
    }
}
