//! Golden tests over the reference example: full parse -> resolve -> layout ->
//! scene pipeline, plus library resolution and layout stability.

use gridflow_core::ast::{ArrowKind, Shape};
use gridflow_core::document::Document;
use gridflow_core::export::{build_scene, svg::to_svg, SceneStyle};
use gridflow_core::geometry::MonoMeasurer;
use gridflow_core::layout::LayoutEngine;
use gridflow_core::library::{LibraryProvider, NoLibrary};
use gridflow_core::model::Placement;
use gridflow_core::ops::{Intent, Op, TextEdit};

const EXAMPLE: &str = include_str!("../../../examples/deploy-pipeline.gfd");

fn doc(src: &str) -> Document {
    Document::new(src.to_string(), Box::new(NoLibrary))
}

#[test]
fn reference_example_parses_clean() {
    let d = doc(EXAMPLE);
    let m = d.model();
    assert_eq!(
        m.diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>(),
        Vec::<String>::new()
    );
    for id in ["commit", "ci", "gate", "fix", "auth", "pay", "stage", "smoke", "prod", "done"] {
        assert!(m.nodes.contains_key(id), "missing node {id}");
        assert!(!m.nodes[id].phantom, "{id} should not be phantom");
    }
    assert_eq!(m.edges.len(), 10);
    assert_eq!(m.groups.len(), 1);
    assert_eq!(m.groups["deploy"].members.len(), 3);

    // Variables and classes resolved.
    assert_eq!(m.nodes["gate"].shape, Shape::Diamond);
    assert_eq!(m.nodes["fix"].shape, Shape::Rounded);
    assert_eq!(m.nodes["fix"].style.fill.unwrap().0, [0xff, 0x6b, 0x35, 0xff]);
    assert_eq!(m.nodes["auth"].label.as_deref(), Some("Auth"));
    assert_eq!(m.nodes["pay"].style.fill.unwrap().0, [0xff, 0xe0, 0xb2, 0xff]);
    assert_eq!(m.nodes["auth"].ports.len(), 2);

    // Placements.
    assert!(matches!(m.nodes["gate"].placement, Placement::Absolute { .. }));
    assert!(matches!(m.nodes["fix"].placement, Placement::Relative { .. }));
    assert!(matches!(m.nodes["commit"].placement, Placement::Auto));

    // Edge details.
    let no_edge = m.edges.iter().find(|e| e.label.as_deref() == Some("no")).unwrap();
    assert!(no_edge.dashed);
    let retry = m.edges.iter().find(|e| e.label.as_deref() == Some("retry")).unwrap();
    assert_eq!(retry.arrow, ArrowKind::Dotted);
    let inv = m.edges.iter().find(|e| e.label.as_deref() == Some("invoices")).unwrap();
    assert_eq!(inv.from_port.as_deref(), Some("out"));
    assert_eq!(inv.to_port.as_deref(), Some("in"));
}

#[test]
fn layout_places_everything_and_honors_pins() {
    let d = doc(EXAMPLE);
    let mut engine = LayoutEngine::new();
    let layout = engine.compute(d.model(), &MonoMeasurer);
    assert!(layout.diagnostics.is_empty(), "{:?}", layout.diagnostics);
    for id in d.model().nodes.keys() {
        assert!(layout.positions.contains_key(id), "no position for {id}");
        assert!(layout.sizes.contains_key(id), "no size for {id}");
    }
    // Pinned node exactly where the source says.
    assert_eq!(layout.positions["gate"], [320.0, 260.0]);
    // Relative: fix is right of gate with the requested gap.
    let (g, gs) = (layout.positions["gate"], layout.sizes["gate"]);
    let (f, fs) = (layout.positions["fix"], layout.sizes["fix"]);
    assert!((f[0] - (g[0] + gs[0] / 2.0 + 80.0 + fs[0] / 2.0)).abs() < 0.5);
    assert_eq!(f[1], g[1]);
    // Group frame exists and contains its members.
    let rect = layout.group_rects["deploy"];
    for m in &d.model().groups["deploy"].members {
        let p = layout.positions[m];
        assert!(p[0] >= rect.min[0] && p[0] <= rect.max[0], "{m} outside group x");
        assert!(p[1] >= rect.min[1] && p[1] <= rect.max[1], "{m} outside group y");
    }
}

#[test]
fn label_edit_does_not_move_nodes() {
    let mut d = doc(EXAMPLE);
    let mut engine = LayoutEngine::new();
    let before = engine.compute(d.model(), &MonoMeasurer);

    // Change one letter inside a label (same length => same measured size).
    let at = d.text().find("Push to main").unwrap();
    d.apply(Op {
        edits: vec![TextEdit::new(at, at + 1, "B")],
        intent: Intent::Typing,
        actor: 0,
        seq: 0,
    });
    let after = engine.compute(d.model(), &MonoMeasurer);
    for (id, pos) in &before.positions {
        let a = after.positions[id];
        // Group translation re-applies each pass; allow float epsilon only.
        assert!(
            (a[0] - pos[0]).abs() < 0.01 && (a[1] - pos[1]).abs() < 0.01,
            "{id} moved after a label-only edit: {pos:?} -> {a:?}"
        );
    }
}

#[test]
fn scene_and_svg_export() {
    let d = doc(EXAMPLE);
    let mut engine = LayoutEngine::new();
    let layout = engine.compute(d.model(), &MonoMeasurer);
    let scene = build_scene(d.model(), &layout, SceneStyle::default(), &MonoMeasurer);
    assert!(scene.items.len() > 20);
    let svg = to_svg(&scene);
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Push to main"));
    assert!(svg.contains("polygon")); // diamond + arrowheads
    assert!(svg.ends_with("</svg>\n"));
}

struct OneFileLib(&'static str);
impl LibraryProvider for OneFileLib {
    fn load(&self, name: &str) -> Option<String> {
        (name == "people").then(|| self.0.to_string())
    }
    fn list(&self) -> Vec<String> {
        vec!["people".into()]
    }
}

#[test]
fn use_pulls_classes_from_library() {
    let lib = "accent = #e3f2fd\nclass Person(name) {\n  shape = [rounded]\n  label = $name\n  fill = $accent\n  port head top\n}\n";
    let d = Document::new(
        "use people\nalice = Person(\"Alice\")\nbob = Person(\"Bob\") right-of alice\nalice.head -> bob.head\n".to_string(),
        Box::new(OneFileLib(lib)),
    );
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.nodes["alice"].label.as_deref(), Some("Alice"));
    assert_eq!(m.nodes["alice"].style.fill.unwrap().0, [0xe3, 0xf2, 0xfd, 0xff]);
    assert_eq!(m.nodes["bob"].ports.len(), 1);
}

#[test]
fn missing_library_is_a_diagnostic_not_a_crash() {
    let d = doc("use nope\nnode a\n");
    assert!(d.model().nodes.contains_key("a"));
    assert!(d.model().diagnostics.iter().any(|x| x.message.contains("nope")));
}

#[test]
fn one_dollar_var_per_node_enforced() {
    let d = doc("a = [rounded]\nb = [bold]\nn$a$b \"X\"\n");
    let m = d.model();
    assert!(m.diagnostics.iter().any(|x| x.message.contains("one `$var`")));
    // First bundle still applied.
    assert_eq!(m.nodes["n"].shape, Shape::Rounded);
}

#[test]
fn all_shapes_parse_size_and_anchor() {
    use gridflow_core::geometry::{boundary_anchor, hit_test, node_size, MonoMeasurer};
    let src = "a [rect]\nb [rounded]\nc [stadium]\nd [circle]\ne [ellipse]\nf [diamond]\ng [hexagon]\nh [parallelogram]\ni [trapezoid]\nj [cylinder]\nk [card]\nl [db]\n";
    let d = doc(src);
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.nodes["d"].shape, Shape::Circle);
    assert_eq!(m.nodes["l"].shape, Shape::Cylinder); // `db` alias
    for node in m.nodes.values() {
        let size = node_size(node, &MonoMeasurer);
        assert!(size[0] > 0.0 && size[1] > 0.0);
        // The center must hit, a faraway point must not, and the boundary
        // anchor must lie strictly between them.
        let c = [100.0, 50.0];
        assert!(hit_test(node.shape, c, size, c), "{:?} center miss", node.shape);
        let far = [c[0] + size[0] * 3.0, c[1] + size[1] * 2.0];
        assert!(!hit_test(node.shape, c, size, far), "{:?} far hit", node.shape);
        let a = boundary_anchor(node.shape, c, size, far);
        assert!(a[0] > c[0] && a[0] < far[0], "{:?} anchor x {a:?}", node.shape);
    }
}

#[test]
fn chained_and_reversed_edges() {
    let d = doc("a -> b -> c : \"hop\" [dashed]\nx <- y\np <.. q\n");
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.edges.len(), 4);
    // Chain: both segments carry the label + attrs.
    assert_eq!(m.edges[0].from.as_str(), "a");
    assert_eq!(m.edges[0].to.as_str(), "b");
    assert_eq!(m.edges[1].from.as_str(), "b");
    assert_eq!(m.edges[1].to.as_str(), "c");
    assert!(m.edges.iter().take(2).all(|e| e.label.as_deref() == Some("hop") && e.dashed));
    // Reversed arrows swap endpoints.
    assert_eq!((m.edges[2].from.as_str(), m.edges[2].to.as_str()), ("y", "x"));
    assert_eq!((m.edges[3].from.as_str(), m.edges[3].to.as_str()), ("q", "p"));
    assert_eq!(m.edges[3].arrow, ArrowKind::Dotted);
}

#[test]
fn default_attrs_apply_forward_only() {
    let src = "before\ndefault node [rounded, fill=#eee]\ndefault edge [dashed]\nafter\nafter2 [rect]\nbefore -> after\n";
    let d = doc(src);
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.nodes["before"].shape, Shape::Rect);
    assert!(m.nodes["before"].style.fill.is_none());
    assert_eq!(m.nodes["after"].shape, Shape::Rounded);
    assert!(m.nodes["after"].style.fill.is_some());
    // Inline attrs still override the default.
    assert_eq!(m.nodes["after2"].shape, Shape::Rect);
    assert!(m.edges[0].dashed);
}

#[test]
fn text_color_attr() {
    let d = doc("accent = #ff6b35\nn \"X\" [text=$accent]\n");
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.nodes["n"].style.text.unwrap().0, [0xff, 0x6b, 0x35, 0xff]);
}

#[test]
fn relative_path_use() {
    let dir = std::env::temp_dir().join(format!("gridflow-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("local.gfd"), "class Box(t) { label = $t }\n").unwrap();
    let lib = gridflow_core::library::FsLibrary::with_doc_dir(
        dir.join("no-global-lib"),
        Some(dir.clone()),
    );
    let d = Document::new(
        "use \"./local.gfd\"\nb = Box(\"hi\")\n".to_string(),
        Box::new(lib),
    );
    assert!(d.model().diagnostics.is_empty(), "{:?}", d.model().diagnostics);
    assert_eq!(d.model().nodes["b"].label.as_deref(), Some("hi"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn phantom_nodes_render_dashed_until_declared() {
    let d = doc("a -> ghost\n");
    let m = d.model();
    assert!(m.nodes["ghost"].phantom);
    assert!(m.nodes["ghost"].style.dashed);
    assert!(m.nodes["a"].phantom);
}

// ---- v0.2 additions -------------------------------------------------------

const GALLERY: &str = include_str!("../../../examples/shapes-gallery.gfd");
const MERMAID_EXAMPLE: &str = include_str!("../../../examples/mermaid-onboarding.mmd");

#[test]
fn shapes_gallery_covers_every_shape_and_parses_clean() {
    let d = doc(GALLERY);
    let m = d.model();
    assert!(
        m.diagnostics.is_empty(),
        "{:?}",
        m.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    // Every shape in the enum appears somewhere in the gallery.
    for (shape, kw) in Shape::ALL {
        assert!(
            m.nodes.values().any(|n| n.shape == *shape),
            "gallery is missing shape `{kw}`"
        );
    }
    // Icons resolved to glyphs and reach the display label.
    let svc = &m.nodes["svc"];
    assert_eq!(svc.style.icon.as_deref(), Some("🔒"));
    assert!(svc.display_label().starts_with("🔒 "));
}

#[test]
fn mermaid_example_runs_the_full_pipeline() {
    let d = doc(MERMAID_EXAMPLE);
    assert_eq!(d.language(), gridflow_core::document::Language::Mermaid);
    let m = d.model();
    let errors: Vec<_> = m
        .diagnostics
        .iter()
        .filter(|d| d.severity == gridflow_core::ast::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(m.nodes.len() >= 10);
    assert_eq!(m.groups.len(), 1);
    assert_eq!(m.nodes["db"].shape, Shape::Cylinder);
    assert_eq!(m.nodes["active"].shape, Shape::DblCircle);
    // classDef + class landed.
    assert!(m.nodes["create"].style.fill.is_some());

    // Layout + scene + SVG work on a mermaid-built model.
    let mut engine = LayoutEngine::new();
    let layout = engine.compute(m, &MonoMeasurer);
    assert_eq!(layout.positions.len(), m.nodes.len());
    let scene = build_scene(m, &layout, SceneStyle::default(), &MonoMeasurer);
    let svg = to_svg(&scene);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("users")); // node label made it to the SVG
}

#[test]
fn mermaid_convert_to_gfd_preserves_structure() {
    let d = doc(MERMAID_EXAMPLE);
    let gfd = gridflow_core::mermaid::to_gfd(d.model());
    let d2 = doc(&gfd);
    assert_eq!(d2.language(), gridflow_core::document::Language::Gfd);
    let errors: Vec<_> = d2
        .model()
        .diagnostics
        .iter()
        .filter(|x| x.severity == gridflow_core::ast::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "converted GFD has errors: {errors:?}\n---\n{gfd}");
    assert_eq!(d2.model().nodes.len(), d.model().nodes.len());
    assert_eq!(d2.model().edges.len(), d.model().edges.len());
    assert_eq!(d2.model().groups.len(), d.model().groups.len());
}

#[test]
fn bt_and_rl_directions_reverse_flow() {
    let tb = doc("dir: TB\na -> b\n");
    let bt = doc("dir: BT\na -> b\n");
    let mut engine = LayoutEngine::new();
    let l_tb = engine.compute(tb.model(), &MonoMeasurer);
    let mut engine2 = LayoutEngine::new();
    let l_bt = engine2.compute(bt.model(), &MonoMeasurer);
    assert!(l_tb.positions["a"][1] < l_tb.positions["b"][1], "TB: a above b");
    assert!(l_bt.positions["a"][1] > l_bt.positions["b"][1], "BT: a below b");

    let rl = doc("dir: RL\na -> b\n");
    let mut engine3 = LayoutEngine::new();
    let l_rl = engine3.compute(rl.model(), &MonoMeasurer);
    assert!(l_rl.positions["a"][0] > l_rl.positions["b"][0], "RL: a right of b");
}

#[test]
fn arrow_aliases_match_canonical_arrows() {
    let d = doc("a --> b\nc <-- d\ne <--> f\n");
    let m = d.model();
    assert!(m.diagnostics.is_empty(), "{:?}", m.diagnostics);
    assert_eq!(m.edges[0].arrow, ArrowKind::Directed);
    assert_eq!((m.edges[1].from.as_str(), m.edges[1].to.as_str()), ("d", "c"));
    assert_eq!(m.edges[2].arrow, ArrowKind::Bidirectional);
}
