# Shapes

Eleven shapes, chosen to cover the classic flowchart vocabulary. The keyword
goes in a node's attribute list: `n "Label" [hexagon]`.

| Keyword | Alias | Shape | Conventional meaning |
|---|---|---|---|
| `rect` | | sharp rectangle | generic step (default) |
| `rounded` | | rounded rectangle | process / action |
| `stadium` | | pill / capsule | start & end points |
| `circle` | | perfect circle | connectors, states |
| `ellipse` | | oval | terminal state |
| `diamond` | | rhombus | decision |
| `hexagon` | | six-sided | preparation / gateway |
| `parallelogram` | `para` | slanted rect | input / output |
| `trapezoid` | | narrow top | manual operation |
| `cylinder` | `db` | database drum | data store |
| `card` | | cut-corner rect | file / artifact |

```gfd
// A quick gallery — paste this into gridflow:
dir: LR
a "rect"          [rect]
b "rounded"       [rounded]       right-of a gap 40
c "stadium"       [stadium]       right-of b gap 40
d "circle"        [circle]        below a gap 60
e "ellipse"       [ellipse]       right-of d gap 40
f "diamond"       [diamond]       right-of e gap 40
g "hexagon"       [hexagon]       below d gap 60
h "parallelogram" [para]          right-of g gap 40
i "trapezoid"     [trapezoid]     right-of h gap 40
j "database"      [db]            below g gap 60
k "card"          [card]          right-of j gap 40
```

## Sizing

Every shape sizes itself so the label fits inside with comfortable padding
(multi-line labels via `\n` are supported). Wide shapes like the diamond and
hexagon grow more than rectangles for the same label — that is intentional,
the label must fit *inside the geometry*, not just the bounding box.

Override with `w=` / `h=`:

```gfd
n "Fixed size" [rounded, w=200, h=80]
```

`circle` keeps itself perfectly round (width = height) unless you override
both.

## Edge anchoring

Edges attach to the exact geometric boundary of the shape — the point where
the line from center to center crosses the outline. A `diamond`'s edges meet
its points and slopes, a `cylinder`'s meet its curved caps. Named ports (from
classes) override this with fixed positions on a chosen side.

## Styling recap

```gfd
n "Styled" [hexagon, fill=#e0f2f1, stroke=#00695c, text=#004d40, bold]
m "Ghost"  [rounded, dashed]
```

All shapes respect `fill`, `stroke`, `text`, `dashed`, `bold`, `w`, `h`.
Phantom nodes (edge references with no declaration) always render dashed with
the id as their label, regardless of defaults.
