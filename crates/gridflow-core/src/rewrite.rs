//! Surgical text rewriting for canvas→text sync. Only ever splices spans the
//! parser recorded, so comments and formatting elsewhere survive by
//! construction.

use crate::model::{DiagramModel, Node, Placement};
use crate::ops::TextEdit;

fn fmt_coord(v: f32) -> String {
    format!("{}", v.round() as i64)
}

pub fn placement_clause(x: f32, y: f32) -> String {
    format!("@ ({}, {})", fmt_coord(x), fmt_coord(y))
}

/// The edit that pins `node` at world position (x, y).
///
/// - Pinned node: its `@` clause span is replaced in place.
/// - Relative hint: the hint clause is replaced by an `@` clause (dragging
///   converts a hint into a pin — predictable, undoable).
/// - Auto: an `@` clause is inserted at the recorded end-of-statement offset.
/// - Phantom (edge-only): a whole declaration line is appended to `src`.
pub fn move_node_edit(src: &str, node: &Node, x: f32, y: f32) -> TextEdit {
    let clause = placement_clause(x, y);
    match &node.placement {
        Placement::Absolute { span, .. } | Placement::Relative { span, .. } => {
            TextEdit::new(span.start, span.end, clause)
        }
        Placement::Auto if node.phantom => {
            let mut insert = String::new();
            if !src.is_empty() && !src.ends_with('\n') {
                insert.push('\n');
            }
            insert.push_str(&format!("{} {}\n", node.id, clause));
            TextEdit::new(src.len(), src.len(), insert)
        }
        Placement::Auto => {
            TextEdit::new(
                node.insert_placement_at,
                node.insert_placement_at,
                format!(" {clause}"),
            )
        }
    }
}

/// Same idea for groups (dragging a group's frame moves the whole cluster).
pub fn move_group_edit(model: &DiagramModel, group_id: &str, x: f32, y: f32) -> Option<TextEdit> {
    let group = model.groups.get(group_id)?;
    let clause = placement_clause(x, y);
    Some(match &group.placement {
        Placement::Absolute { span, .. } | Placement::Relative { span, .. } => {
            TextEdit::new(span.start, span.end, clause)
        }
        Placement::Auto => TextEdit::new(
            group.insert_placement_at,
            group.insert_placement_at,
            format!(" {clause}"),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;
    use crate::library::NoLibrary;
    use crate::ops::{Intent, Op};

    fn doc(src: &str) -> Document {
        Document::new(src.to_string(), Box::new(NoLibrary))
    }

    fn drag(d: &mut Document, id: &str, x: f32, y: f32) {
        let node = d.model().nodes.get(id).cloned().expect("node exists");
        let edit = move_node_edit(d.text(), &node, x, y);
        d.apply(Op { edits: vec![edit], intent: Intent::MoveNode(node.id.clone()), actor: 0, seq: 0 });
    }

    #[test]
    fn drag_auto_node_inserts_clause_before_comment() {
        let mut d = doc("node a \"Hello\" // trailing comment\nnode b\n");
        drag(&mut d, "a", 120.0, 40.0);
        assert_eq!(d.text(), "node a \"Hello\" @ (120, 40) // trailing comment\nnode b\n");
    }

    #[test]
    fn drag_pinned_node_replaces_in_place() {
        let mut d = doc("a \"X\" @ (10, 20) // keep me\n");
        drag(&mut d, "a", -5.0, 99.6);
        assert_eq!(d.text(), "a \"X\" @ (-5, 100) // keep me\n");
    }

    #[test]
    fn drag_relative_node_converts_hint_to_pin() {
        let mut d = doc("node a\nnode b right-of a gap 40\n");
        drag(&mut d, "b", 300.0, 60.0);
        assert_eq!(d.text(), "node a\nnode b @ (300, 60)\n");
    }

    #[test]
    fn drag_phantom_appends_declaration() {
        let mut d = doc("a -> ghost");
        drag(&mut d, "ghost", 50.0, 70.0);
        assert_eq!(d.text(), "a -> ghost\nghost @ (50, 70)\n");
        assert!(!d.model().nodes["ghost"].phantom);
    }

    #[test]
    fn drag_is_idempotent_and_undoable() {
        let mut d = doc("node a \"Hi\"\n");
        drag(&mut d, "a", 10.0, 10.0);
        drag(&mut d, "a", 200.0, 300.0);
        assert_eq!(d.text(), "node a \"Hi\" @ (200, 300)\n");
        d.undo(); // both drags coalesce (same node moved consecutively)
        assert_eq!(d.text(), "node a \"Hi\"\n");
    }
}
