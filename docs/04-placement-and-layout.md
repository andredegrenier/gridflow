# Placement & Layout

gridflow has a three-tier placement model. Every node is in exactly one tier,
and you can move nodes between tiers just by editing text (or dragging).

## Tier 1 — Auto (no clause)

Nodes without a placement clause are laid out automatically with a layered
("Sugiyama-style") algorithm: edges define layers along the flow direction
(`dir: TB` or `LR`), nodes are ordered within layers to reduce crossings, and
spacing is derived from node sizes.

**Stability guarantee:** the auto-layout only recomputes when the diagram's
*structure* changes (nodes, edges, sizes, groupings — not label edits, and not
pinned coordinates). When it does recompute, it seeds from the previous
positions so nodes don't teleport while you type.

## Tier 2 — Pinned: `@ (x, y)`

```gfd
gate "Tests pass?" [diamond] @ (320, 260)
```

Pinned nodes sit at exactly those world coordinates (the node's center),
forever. **Dragging any node on the canvas pins it** — the drag writes the
`@` clause into the text at the end of the node's statement. Dragging an
already-pinned node just rewrites the numbers. Undo (⌘Z) restores both the
position and the text.

The coordinate system: x grows right, y grows down, units are the same as
node sizes (a typical node is ~60–160 wide). `(0, 0)` is wherever you decide
it is — only relative positions matter, and `Shift+F` re-frames everything.

## Tier 3 — Relative hints

```gfd
fix "Fix the build" right-of gate gap 80
legend above title
note left-of legend        // default gap is 50
```

`right-of` / `left-of` / `above` / `below` place a node a fixed gap from an
**anchor** node's final position — whether that anchor is auto-laid, pinned,
or itself relative (chains resolve in dependency order; cycles are diagnosed
and fall back to auto).

This is the tool for the classic mermaid frustration: *"I just want these two
unconnected boxes side by side."* No invisible edges, no hacks:

```gfd
box1 "Service A"
box2 "Service B" right-of box1 gap 60   // no edge needed
```

Dragging a relative-placed node **converts the hint to a pin** (the clause is
rewritten to `@ (x, y)`), because after a manual move the hint no longer
describes reality. Undo restores the hint.

## Groups

```gfd
group infra "Infrastructure" {
  dir: LR
  node lb "Load balancer"
  node app "App server" right-of lb gap 50
  lb -> app
}
```

- Members lay out as their own cluster, with an optional local `dir`.
- An **auto** group participates in the top-level layout as one supernode
  (edges to members pull the whole cluster).
- A **pinned** group (`@ (x, y)`) centers the cluster at that point.
- A **relative** group anchors its frame off any node.
- Members with their own `@` pins are never moved by the group — an explicit
  coordinate always wins over everything.
- Drag the group's title bar to move the whole cluster (writes the group's
  placement clause).

## Fit & navigation

| Key | Action |
|---|---|
| `F` | fit the current selection (or all, if nothing selected) |
| `Shift+F` | fit the whole diagram |
| `⌘J` | center the camera on the node under the text cursor |
| double-click a node | jump the text cursor to its declaration |

## Practical workflow

1. Type the nodes and edges free-form; let auto-layout arrange them.
2. When the overall shape is right, drag the anchors you care about — they pin.
3. Use relative hints for satellites (legends, notes, side-by-side pairs).
4. Leave everything else auto so future nodes flow in without manual work.
