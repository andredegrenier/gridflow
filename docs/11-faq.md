# FAQ & Troubleshooting

**Why `//` comments instead of `#` like my mermaid files?**
`#` introduces colors (`#ff6b35`). A lexer can't reliably tell `#fdd` (a
color) from `#fdd` (a comment about fudge), so `#` is colors, `//` is
comments. It's the one place GFD deliberately differs from mermaid habits.

**My node didn't move when I edited its label. Bug?**
Feature — layout only recomputes on *structural* change (nodes/edges/sizes).
Label edits that change the node's size do trigger a relayout, seeded from
the old positions, so movement is minimal.

**I dragged a node and now it ignores auto-layout.**
Dragging pins (`@ (x, y)` in the text). Delete the clause to return the node
to auto-layout. Undo right after the drag does the same.

**Two unconnected nodes side by side?**
`b right-of a gap 60` — no edge needed. See Placement & Layout.

**A red `library not found` on my `use` line.**
Bare names load from the global library dir (see Libraries for the path per
OS); quoted paths load relative to the *saved* document — an unsaved buffer
has no folder, so save first.

**`X is already defined (single assignment)`**
Variables bind once. Rename, or compose (`b = [$a, ...]`) instead of
reassigning.

**Why can't I write `person$a$b`?**
One bundle per node keeps precedence trivial to reason about. Compose:
`ab = [$a, $b]` then `person$ab`.

**Port arrows attach to weird places.**
Check the port's declared side — `port out right` anchors mid-right edge.
Several ports on one side spread evenly in declaration order. Undeclared port
names fall back to boundary anchoring (with a diagnostic).

**Where did my window layout / recent files go?**
They're stored via the OS config dir; deleting gridflow's config folder
resets UI state (your `.gfd` files are of course untouched).

**Big diagram feels slow when zoomed way out.**
Below ~5px text, labels render as bars, which is the fast path. If it's still
slow, check you're not running a debug build — use `cargo run --release`.

**Does it run on Windows/Linux?**
Yes — pure-Rust stack (eframe/wgpu). Portable Windows builds are attached to
GitHub releases; or `cargo build --release` on any OS.

**Can I use it headless (CI) to render SVGs?**
Not yet as a CLI, but `gridflow-core` exposes the full pipeline
(parse → resolve → layout → Scene → SVG) with no GUI dependency — a `gfd2svg`
binary is a ~30-line contribution. PRs welcome.

**When is multiplayer coming?**
Phase 2: automerge-based sync with a small relay server (AWS-hostable). The
document/op model in v1 was designed for it — see Architecture.
