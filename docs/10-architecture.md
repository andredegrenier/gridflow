# Architecture (for contributors)

Two crates; strict dependency direction (`app → core`, never back):

```
crates/gridflow-core     no GUI dependencies — parsing, resolution, layout,
                         document/ops, rewriting, export. Tests run in ms.
crates/gridflow-app      the eframe/egui shell.
```

## Data flow (one frame)

```
keystrokes → TextEdit scratch → diff → Op{Typing} ─┐
canvas drag-end → rewrite::move_node_edit → Op{MoveNode} ─┤
                                                     ▼
                                       Document::apply(op)      ← THE only
                                        ├─ splice text             mutation
                                        ├─ record inverse (undo)    path
                                        └─ parse → resolve → DiagramModel
                                                     ▼
                              LayoutEngine::compute(model)  (hash-gated)
                                                     ▼
                              canvas paint  /  SVG export (same Scene geometry)
```

## Core modules

| Module | Responsibility |
|---|---|
| `lexer` | logos tokens with byte spans; also feeds editor highlighting |
| `parser` | recursive descent; per-statement recovery; spans on everything |
| `resolve` | variables, bundles, classes, ports, defaults, `use` imports, phantom nodes |
| `model` | the resolved `DiagramModel` (nodes/edges/groups + diagnostics) |
| `layout` | layered auto-layout with pinned constraints, relative hints, groups as supernodes; structural-hash caching + previous-position seeding |
| `geometry` | shape outlines, sizing, boundary/port anchors, hit tests — the single geometry source for canvas, export, and anchoring |
| `document` | text + model + unified undo/redo; `apply(Op)` is the sole mutation path |
| `ops` | `Op { edits, intent, actor, seq }` — serializable; the collaboration seam |
| `rewrite` | drag → minimal text splice (placement clause only) |
| `export` | Scene (styled primitives) → SVG string |

## Invariants worth knowing

1. **Nothing mutates `Document`'s text except `apply`.** Undo entries are
   (inverse, forward) batch sequences; the text alternates between exactly two
   states across undo/redo, so recorded edits stay valid.
2. **Spans are never stale** — every `apply` reparses, so all spans in the
   current model refer to the current text.
3. **Layout stability** — the layered pass only runs when the structural hash
   changes; pinned coordinates are excluded from the hash so dragging never
   relayouts the rest.
4. **One geometry source** — `geometry::shape_outline` drives canvas fill,
   stroke, hit-testing, edge anchoring, and SVG export identically.
5. **Errors degrade per statement** in both parser and resolver; the model is
   always renderable.

## Collaboration roadmap (phase 2)

Because canvas edits lower to text edits, multiuser gridflow is exactly the
collaborative-text problem, which automerge solves wholesale:

- `DocumentStore` trait: swap the `String`-backed store for an automerge
  `Text`-backed one; `TextEdit` maps 1:1 onto `splice_text`.
- `gridflow-sync` crate: a small WebSocket relay (planned: axum on AWS) using
  automerge's sync protocol; presence = cursor byte offsets, which the editor
  and canvas can both already visualize.
- NodeIds are user-authored, so there is no cross-peer id mapping problem.

## Testing

```sh
cargo test -p gridflow-core
```

- Unit tests in each module (lexer spans, undo round-trips, rewrite cases).
- `tests/golden.rs` — the reference example through the full pipeline, layout
  invariants, library resolution, new-syntax coverage.
- `tests/prop_rewrite.rs` — property tests: the parser never panics on
  arbitrary input; drag-writeback touches only the placement clause; undo
  restores exact bytes.
