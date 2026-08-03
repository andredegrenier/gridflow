//! Resolved diagram model: what the layout engine and renderer consume.
//! Built from the AST by `resolve`; carries spans so canvas interactions can
//! write back into the source text.

use crate::ast::{ArrowKind, Diagnostic, Dir, LayoutDir, Shape, Side, Span};
use indexmap::IndexMap;
use smol_str::SmolStr;

pub type NodeId = SmolStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Rgba([r, g, b, 255])
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeStyle {
    pub fill: Option<Rgba>,
    pub stroke: Option<Rgba>,
    /// Label color.
    pub text: Option<Rgba>,
    pub dashed: bool,
    pub bold: bool,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub name: SmolStr,
    pub side: Side,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Placement {
    Auto,
    /// Span covers exactly the `@ (x, y)` clause in the source.
    Absolute { pos: [f32; 2], span: Span },
    /// Span covers exactly the relative clause in the source.
    Relative { anchor: NodeId, dir: Dir, gap: Option<f32>, span: Span },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: Option<String>,
    pub shape: Shape,
    pub style: NodeStyle,
    pub ports: Vec<Port>,
    pub placement: Placement,
    pub group: Option<NodeId>,
    /// Whole statement span (for jump-to-text and phantom declaration).
    pub stmt_span: Span,
    /// Byte offset where a placement clause can be inserted (end of statement).
    pub insert_placement_at: usize,
    /// True when the node exists only because an edge references it.
    pub phantom: bool,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: NodeId,
    pub from_port: Option<SmolStr>,
    pub to: NodeId,
    pub to_port: Option<SmolStr>,
    pub arrow: ArrowKind,
    pub label: Option<String>,
    pub dashed: bool,
    pub bold: bool,
    pub stroke: Option<Rgba>,
    pub stmt_span: Span,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub id: NodeId,
    pub label: Option<String>,
    pub placement: Placement,
    pub dir: Option<LayoutDir>,
    pub members: Vec<NodeId>,
    pub stmt_span: Span,
    pub insert_placement_at: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DiagramModel {
    pub nodes: IndexMap<NodeId, Node>,
    pub edges: Vec<Edge>,
    pub groups: IndexMap<NodeId, Group>,
    pub direction: LayoutDir,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagramModel {
    /// Node whose statement contains the byte offset (for cursor→node jump).
    pub fn node_at_byte(&self, offset: usize) -> Option<&NodeId> {
        self.nodes
            .values()
            .filter(|n| !n.phantom && n.stmt_span.contains(offset))
            .map(|n| &n.id)
            .next()
    }
}
