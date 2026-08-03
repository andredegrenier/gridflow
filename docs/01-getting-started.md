# Getting Started

gridflow is a native diagram editor built around a plain-text language called
**GFD** (gridflow diagram). You describe nodes and edges in text; gridflow lays
them out, and anything you drag on the canvas is written back into the text as
exact coordinates. The text file is always the single source of truth — perfect
for git, code review, and (soon) real-time collaboration.

## Your first diagram

Open gridflow and replace the editor contents with:

```gfd
node start "Request comes in"
node auth  "Authenticated?" [diamond]
node ok    "Serve page" [rounded, fill=#e8f5e9]
node deny  "401" [stadium, fill=#ffebee]

start -> auth
auth -> ok   : "yes"
auth -> deny : "no"
```

The canvas updates on every keystroke. Things to try immediately:

1. **Zoom** with pinch or ⌘-scroll — labels stay razor sharp at any zoom
   because gridflow re-renders glyphs at the current scale instead of scaling
   a bitmap.
2. **Drag a node.** When you release it, look at the text: an `@ (x, y)`
   clause appeared at the end of its line. That node is now *pinned* — the
   auto-layout will never move it again. ⌘Z undoes both the move and the text.
3. **Break a line on purpose** (type garbage in the middle of the file). Only
   that statement gets a red underline; the rest of the diagram keeps
   rendering. Fix it and everything snaps back.

## The three panes

| Pane | Toggle | Purpose |
|---|---|---|
| Editor (left) | ⌘1 | The GFD source, with highlighting and error underlines |
| Canvas (center) | — | Pan/zoom view; drag to rearrange |
| Notes (right) | ⌘2 | A markdown sidecar file (`<name>.notes.md`) for context |
| Library (right) | ⌘3 | Browse/import the global class library |

## Saving

⌘S writes the `.gfd` file (and the notes sidecar). The file format *is* the
language — there is no binary project file. Anyone with gridflow (or a text
editor) can open it.

## Where to go next

- **Language Reference** — every statement and attribute.
- **Shapes** — the full catalog with pictures of the syntax.
- **Placement & Layout** — how auto-layout, pins, and hints interact.
- **Variables & Classes** — make diagrams that restyle themselves.
- **Libraries** — share classes across files and machines.
- **Cookbook** — complete worked examples.
