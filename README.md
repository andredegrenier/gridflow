# gridflow

A native, keyboard-friendly diagram app with a plain-text language (**GFD**)
built around explicit layout — born from three mermaid frustrations:

- **Readable at any size** — pan/zoom canvas where text re-rasterizes at every
  zoom level (never scaled bitmaps). 350-line diagrams stay crisp at 25%–800%.
- **Precise placement** — pin nodes at coordinates (`@ (320, 260)`) or place
  unconnected nodes relative to each other (`b right-of a gap 60`) with no
  fake edges.
- **Two-way editing** — auto-layout places anything you don't position;
  dragging a node writes `@ (x, y)` back into the source text surgically.
  One undo history covers both text and canvas.

Plus: 11 shapes, variables, parameterized classes with ports, a global +
project-local class library, a markdown notes pane, SVG/PNG export, and full
in-app documentation (`⌘/`). The text file is the only file format — perfect
for git.

```gfd
use flow
dir: TB

in  = Data("Order JSON")
val = Process("Validate")
ok  = Decision("Valid?")
db  = Store("orders")
err "Reject" [stadium, fill=#ffebee] right-of ok gap 80

in -> val -> ok
ok.yes -> db.in
ok.no  -> err : "400" [dashed]
```

## Install / run

**Portable builds** (Windows `.exe`, macOS): grab the latest zip from
[Releases](../../releases) — unzip and run, no installer.

**From source** (any OS, Rust stable):

```sh
cargo run --release -p gridflow-app
```

The app opens with a reference example; `examples/shapes-gallery.gfd` shows
every shape and syntax feature. Tests: `cargo test -p gridflow-core`.

## Documentation

Press **⌘/** in the app (Help → Documentation), or read the same book in
[`docs/`](docs/):

1. [Getting Started](docs/01-getting-started.md)
2. [Language Reference](docs/02-language-reference.md)
3. [Shapes](docs/03-shapes.md)
4. [Placement & Layout](docs/04-placement-and-layout.md)
5. [Variables & Classes](docs/05-variables-and-classes.md)
6. [Libraries](docs/06-libraries.md)
7. [Editor, Canvas & UI](docs/07-editor-canvas-ui.md)
8. [Export](docs/08-export.md)
9. [Cookbook](docs/09-cookbook.md)
10. [Architecture](docs/10-architecture.md)
11. [FAQ](docs/11-faq.md)

## Language at a glance

| Feature | Syntax |
|---|---|
| Nodes | `id "Label" [rounded, fill=#e3f2fd]` |
| Shapes | `rect rounded stadium circle ellipse diamond hexagon parallelogram trapezoid cylinder card` (aliases: `db`, `para`) |
| Edges | `a -> b : "label" [dashed]` · also `<-`, `<->`, `--`, `..>`, `<..` and chains `a -> b -> c` |
| Pin | `@ (x, y)` — written automatically when you drag |
| Relative | `right-of X gap 40`, `left-of`, `above`, `below` |
| Variables | `warn = #ff6b35` · bundles `hot = [rounded, fill=$warn]` · apply `n$hot` · interpolate `"$env api"` |
| Defaults | `default node [rounded]` · `default edge [dashed]` |
| Classes | `class Svc(name, color=#eee) { label = $name; port out right }` → `s = Svc("API")` → `s.out -> t` |
| Groups | `group g "Title" { dir: LR ... }` |
| Libraries | `use flow, aws` (global) · `use "./styles.gfd"` (project-local) |
| Comments | `//` (`#` is reserved for colors) |

## Keyboard

`⌘Z/⇧⌘Z` unified undo · `⌘S/⌘O` save/open · `F`/`⇧F` fit ·
`⌘J` text-cursor → node · double-click node → source line ·
`⌘1/2/3` panes · `⌘/` docs

## Architecture

```
crates/gridflow-core   parser (byte spans, per-statement recovery), resolver,
                       layered layout with pinned constraints, unified op-based
                       document + undo, drag→text rewrite engine, SVG export.
                       No GUI dependencies; 31 tests incl. property tests.
crates/gridflow-app    eframe/egui shell: canvas, highlighting editor, notes,
                       library browser, docs viewer, PNG export (resvg).
```

Every mutation is a serializable `Op` through a single code path — the
designed seam for phase 2: automerge-based real-time collaboration behind an
AWS-hosted relay. See [Architecture](docs/10-architecture.md).

## License

MIT
