//! Raw syntax tree produced by the parser. Every element carries byte spans
//! into the source text so the rewrite engine can splice surgically.

use smol_str::SmolStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
    pub fn contains(&self, offset: usize) -> bool {
        offset >= self.start && offset <= self.end
    }
    pub fn merge(self, other: Span) -> Span {
        Span::new(self.start.min(other.start), self.end.max(other.end))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub severity: Severity,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Diagnostic { span, message: message.into(), severity: Severity::Error }
    }
    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Diagnostic { span, message: message.into(), severity: Severity::Warning }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shape {
    Rect,
    Rounded,
    /// Pill / capsule (mermaid's `([x])`).
    Stadium,
    Circle,
    Ellipse,
    Diamond,
    Hexagon,
    /// Slanted rect — classic input/output.
    Parallelogram,
    /// Narrow top, wide bottom — manual operation.
    Trapezoid,
    /// Database.
    Cylinder,
    /// Rect with a cut top-left corner.
    Card,
}

impl Shape {
    pub const ALL: &'static [(Shape, &'static str)] = &[
        (Shape::Rect, "rect"),
        (Shape::Rounded, "rounded"),
        (Shape::Stadium, "stadium"),
        (Shape::Circle, "circle"),
        (Shape::Ellipse, "ellipse"),
        (Shape::Diamond, "diamond"),
        (Shape::Hexagon, "hexagon"),
        (Shape::Parallelogram, "parallelogram"),
        (Shape::Trapezoid, "trapezoid"),
        (Shape::Cylinder, "cylinder"),
        (Shape::Card, "card"),
    ];

    pub fn from_keyword(word: &str) -> Option<Shape> {
        match word {
            "db" => Some(Shape::Cylinder), // ergonomic alias
            "para" => Some(Shape::Parallelogram),
            _ => Self::ALL
                .iter()
                .find(|(_, kw)| *kw == word)
                .map(|(s, _)| *s),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Above,
    Below,
    LeftOf,
    RightOf,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LayoutDir {
    #[default]
    TopBottom,
    LeftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrowKind {
    Directed,      // ->
    Bidirectional, // <->
    Undirected,    // --
    Dotted,        // ..>
}

/// A string literal, split into literal and `$var` interpolation segments.
#[derive(Debug, Clone, PartialEq)]
pub struct StrLit {
    pub segments: Vec<StrSeg>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrSeg {
    Lit(String),
    Var(SmolStr),
}

/// Right-hand side value expression (assignments, attr values, class vars).
#[derive(Debug, Clone, PartialEq)]
pub enum ValueExpr {
    Color { rgba: [u8; 4], span: Span },
    Num { value: f32, span: Span },
    Str(StrLit),
    VarRef { name: SmolStr, span: Span },
    Bundle { items: Vec<AttrItem>, span: Span },
}

impl ValueExpr {
    pub fn span(&self) -> Span {
        match self {
            ValueExpr::Color { span, .. }
            | ValueExpr::Num { span, .. }
            | ValueExpr::VarRef { span, .. }
            | ValueExpr::Bundle { span, .. } => *span,
            ValueExpr::Str(s) => s.span,
        }
    }
}

/// One item inside `[...]` — node attrs and bundle values share this grammar.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrItem {
    Shape(Shape),
    Dashed,
    Bold,
    Fill(ValueExpr),
    Stroke(ValueExpr),
    /// `text=#color` — label color.
    TextColor(ValueExpr),
    Width(ValueExpr),
    Height(ValueExpr),
    /// `$var` splice of another bundle inside a bundle literal.
    Splice { name: SmolStr, span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlacementAst {
    /// `@ (x, y)` — span covers exactly the clause.
    Absolute { x: f32, y: f32, span: Span },
    /// `right-of other gap 40` — span covers exactly the clause.
    Relative { anchor: SmolStr, anchor_span: Span, dir: Dir, gap: Option<f32>, span: Span },
}

impl PlacementAst {
    pub fn span(&self) -> Span {
        match self {
            PlacementAst::Absolute { span, .. } | PlacementAst::Relative { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UseStmt {
    pub names: Vec<(SmolStr, Span)>,
    pub stmt_span: Span,
}

#[derive(Debug, Clone)]
pub struct DirStmt {
    pub dir: LayoutDir,
    pub stmt_span: Span,
}

/// `name = <value>` or `name = Class(args) [placement]` (node instantiation).
#[derive(Debug, Clone)]
pub struct AssignStmt {
    pub name: SmolStr,
    pub name_span: Span,
    pub rhs: AssignRhs,
    pub placement: Option<PlacementAst>,
    pub stmt_span: Span,
    pub insert_placement_at: usize,
}

#[derive(Debug, Clone)]
pub enum AssignRhs {
    Value(ValueExpr),
    Call { class: SmolStr, class_span: Span, args: Vec<ValueExpr>, span: Span },
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: SmolStr,
    pub default: Option<ValueExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PortDecl {
    pub name: SmolStr,
    pub side: Side,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassStmt {
    pub name: SmolStr,
    pub name_span: Span,
    pub params: Vec<Param>,
    pub vars: Vec<(SmolStr, ValueExpr)>,
    pub ports: Vec<PortDecl>,
    pub stmt_span: Span,
}

#[derive(Debug, Clone)]
pub struct NodeStmt {
    pub id: SmolStr,
    pub id_span: Span,
    /// `person$A` applications; more than one is a diagnostic (resolver enforces).
    pub applied_vars: Vec<(SmolStr, Span)>,
    pub label: Option<StrLit>,
    pub attrs: Vec<AttrItem>,
    pub placement: Option<PlacementAst>,
    pub stmt_span: Span,
    pub insert_placement_at: usize,
}

#[derive(Debug, Clone)]
pub struct EndpointRef {
    pub node: SmolStr,
    pub port: Option<SmolStr>,
    pub span: Span,
}

/// One arrow in an edge chain; `reversed` is true for `<-` (the edge points
/// from the right endpoint to the left one).
#[derive(Debug, Clone, Copy)]
pub struct ArrowTok {
    pub kind: ArrowKind,
    pub reversed: bool,
}

/// `a -> b -> c : "label" [attrs]` — endpoints.len() == arrows.len() + 1.
/// The label/attrs apply to every segment in the chain.
#[derive(Debug, Clone)]
pub struct EdgeStmt {
    pub endpoints: Vec<EndpointRef>,
    pub arrows: Vec<ArrowTok>,
    pub label: Option<StrLit>,
    pub attrs: Vec<AttrItem>,
    pub stmt_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultTarget {
    Node,
    Edge,
}

/// `default node [rounded, fill=#eee]` — baseline attrs for every node/edge
/// declared after this statement.
#[derive(Debug, Clone)]
pub struct DefaultStmt {
    pub target: DefaultTarget,
    pub attrs: Vec<AttrItem>,
    pub stmt_span: Span,
}

#[derive(Debug, Clone)]
pub struct GroupStmt {
    pub id: SmolStr,
    pub id_span: Span,
    pub label: Option<StrLit>,
    pub placement: Option<PlacementAst>,
    pub dir: Option<LayoutDir>,
    pub body: Vec<Stmt>,
    pub stmt_span: Span,
    pub insert_placement_at: usize,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Use(UseStmt),
    Dir(DirStmt),
    Default(DefaultStmt),
    Assign(AssignStmt),
    Class(ClassStmt),
    Node(NodeStmt),
    Edge(EdgeStmt),
    Group(GroupStmt),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::Use(s) => s.stmt_span,
            Stmt::Dir(s) => s.stmt_span,
            Stmt::Default(s) => s.stmt_span,
            Stmt::Assign(s) => s.stmt_span,
            Stmt::Class(s) => s.stmt_span,
            Stmt::Node(s) => s.stmt_span,
            Stmt::Edge(s) => s.stmt_span,
            Stmt::Group(s) => s.stmt_span,
        }
    }
}

/// Result of parsing a whole file.
#[derive(Debug, Clone, Default)]
pub struct Ast {
    pub stmts: Vec<Stmt>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_hex_color(text: &str) -> Option<[u8; 4]> {
    let hex = text.strip_prefix('#')?;
    let d = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    let s = |i: usize| {
        let c = u8::from_str_radix(hex.get(i..i + 1)?, 16).ok()?;
        Some(c * 17)
    };
    match hex.len() {
        3 => Some([s(0)?, s(1)?, s(2)?, 255]),
        4 => Some([s(0)?, s(1)?, s(2)?, s(3)?]),
        6 => Some([d(0)?, d(2)?, d(4)?, 255]),
        8 => Some([d(0)?, d(2)?, d(4)?, d(6)?]),
        _ => None,
    }
}
