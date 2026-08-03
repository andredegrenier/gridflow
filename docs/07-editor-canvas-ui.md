# Editor, Canvas & UI Guide

## Keyboard & mouse reference

### Canvas

| Input | Action |
|---|---|
| pinch / ⌘-scroll | zoom, anchored at the pointer |
| scroll | pan |
| drag empty space | pan |
| drag a node | move it (pins it on release, writing `@ (x, y)`) |
| drag a group's title bar | move the whole group |
| click a node | select (shows its ports) |
| double-click a node | jump the text cursor to its declaration |
| `Esc` | clear selection |
| `F` | fit selection (or all) |
| `Shift+F` | fit all |

### Global

| Key | Action |
|---|---|
| `⌘Z` / `⇧⌘Z` | undo / redo — one history across text edits *and* canvas drags |
| `⌘S` | save (diagram + notes sidecar) |
| `⌘O` | open |
| `⌘J` | center camera on the node under the text cursor |
| `⌘1` / `⌘2` / `⌘3` | toggle editor / notes / library panes |

## The editor pane

- Syntax highlighting driven by the same lexer the parser uses — what looks
  like a keyword *is* a keyword.
- Diagnostics appear as red (error) or amber (warning) underlines, with the
  first message and a total count in the status bar.
- The editor and canvas are two views of one document: a drag on the canvas
  shows up as a text change instantly, and `⌘Z` walks back through both kinds
  of edits in order.

## The canvas

- **Crisp text at any zoom** — glyphs are laid out at the current on-screen
  size every frame (quantized to avoid font-atlas churn). Below ~5px
  effective size, labels draw as placeholder bars — zoom in and they return.
- **Culling** — off-screen nodes are skipped, so very large diagrams stay
  responsive when zoomed in.
- Zoom range is 5%–1600%; the status bar shows the current percentage.

## Notes pane

Each diagram may have a markdown sidecar: `pipeline.gfd` ↔
`pipeline.notes.md`, kept next to the diagram file. The pane renders
GitHub-flavored markdown (tables, code fences, task lists); the Edit/Preview
button toggles a plain-text editor. Notes save together with the diagram on
`⌘S`.

## Status bar

`filename • | N problem(s) <first message> | zoom% | status`

The dirty dot (`•`) means unsaved changes; closing the window with unsaved
changes asks first.

## Persistence

Window size, pane widths, pane visibility, recent files, the last open file,
and the camera position are remembered between launches. Delete the app's
config directory to reset everything.
