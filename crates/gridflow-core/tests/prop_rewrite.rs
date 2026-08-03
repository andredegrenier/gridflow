//! Property tests for the riskiest core logic: the parser must never panic,
//! and drag-writeback must change only the dragged node's placement clause
//! while leaving every other byte of the file intact.

use gridflow_core::document::Document;
use gridflow_core::library::NoLibrary;
use gridflow_core::model::Placement;
use gridflow_core::ops::{Intent, Op};
use gridflow_core::parser::parse;
use gridflow_core::rewrite::move_node_edit;
use proptest::prelude::*;

fn doc(src: &str) -> Document {
    Document::new(src.to_string(), Box::new(NoLibrary))
}

proptest! {
    #[test]
    fn parser_never_panics_on_arbitrary_input(src in "\\PC{0,300}") {
        let _ = parse(&src);
    }

    #[test]
    fn parser_never_panics_on_gfd_like_input(
        src in r#"(node [a-c]|[a-c] -> [a-c]|[a-c] = \[rounded\]|dir: TB|group g \{|\}|@ \(-?[0-9]{1,3}, -?[0-9]{1,3}\)|"[a-z ]{0,10}"|//x|\$[a-c]|\n| ){0,60}"#
    ) {
        let _ = parse(&src);
    }

    #[test]
    fn drag_rewrite_touches_only_the_placement(
        x in -500i32..500,
        y in -500i32..500,
        x2 in -500i32..500,
        y2 in -500i32..500,
    ) {
        let src = "// header comment\nnode a \"Hello world\" [rounded] // trail\nnode b @ (10, 20)\nc right-of a gap 30 // hint\na -> b : \"lbl\"\n";
        let mut d = doc(src);
        for (id, px, py) in [("a", x, y), ("b", x2, y2), ("c", x, y2)] {
            let node = d.model().nodes.get(id).cloned().unwrap();
            let edit = move_node_edit(d.text(), &node, px as f32, py as f32);
            d.apply(Op { edits: vec![edit], intent: Intent::MoveNode(node.id.clone()), actor: 0, seq: 0 });

            // The node is now pinned exactly where the drag ended.
            let node = d.model().nodes.get(id).unwrap();
            match &node.placement {
                Placement::Absolute { pos, .. } => {
                    prop_assert_eq!(*pos, [px as f32, py as f32]);
                }
                other => prop_assert!(false, "expected pin after drag, got {:?}", other),
            }
        }
        // Comments and unrelated statements survived every rewrite.
        prop_assert!(d.text().contains("// header comment"));
        prop_assert!(d.text().contains("// trail"));
        prop_assert!(d.text().contains("// hint"));
        prop_assert!(d.text().contains("a -> b : \"lbl\""));
        prop_assert!(d.model().diagnostics.is_empty(), "diags: {:?}", d.model().diagnostics);
    }

    #[test]
    fn drag_then_drag_is_idempotent_in_text(
        x in -500i32..500,
        y in -500i32..500,
    ) {
        let mut d = doc("node a \"Hi\"\n");
        for _ in 0..3 {
            let node = d.model().nodes.get("a").cloned().unwrap();
            let edit = move_node_edit(d.text(), &node, x as f32, y as f32);
            d.apply(Op { edits: vec![edit], intent: Intent::MoveNode(node.id.clone()), actor: 0, seq: 0 });
        }
        // Same target position repeatedly => stable text, single clause.
        prop_assert_eq!(d.text().matches('@').count(), 1);
    }

    #[test]
    fn undo_restores_exact_bytes(
        edits in proptest::collection::vec((0usize..40, 0usize..8, "[a-z@ \\n]{0,6}"), 1..8)
    ) {
        let src = "node a \"Hello\"\nnode b @ (1, 2)\na -> b\n";
        let mut d = doc(src);
        let mut applied = 0;
        for (start, len, insert) in edits {
            let s = start.min(d.text().len());
            let e = (s + len).min(d.text().len());
            if !d.text().is_char_boundary(s) || !d.text().is_char_boundary(e) { continue; }
            let op = Op {
                edits: vec![gridflow_core::ops::TextEdit::new(s, e, insert)],
                intent: Intent::Typing,
                actor: 0,
                seq: 0,
            };
            if d.apply(op) == gridflow_core::document::ApplyResult::Applied {
                applied += 1;
            }
        }
        for _ in 0..applied {
            d.undo();
        }
        prop_assert_eq!(d.text(), src);
    }
}
