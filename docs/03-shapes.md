# Shapes

Seventeen shapes covering the classic flowchart vocabulary plus state-machine
and annotation staples. The keyword goes in a node's attribute list:
`n "Label" [hexagon]`.

| Keyword | Aliases | Shape | Conventional meaning |
|---|---|---|---|
| `rect` | | sharp rectangle | generic step (default) |
| `rounded` | | rounded rectangle | process / action |
| `stadium` | `pill` | pill / capsule | start & end points |
| `circle` | | perfect circle | connectors, states |
| `ellipse` | `oval` | oval | terminal state |
| `diamond` | `rhombus`, `decision` | rhombus | decision |
| `hexagon` | `hex` | six-sided | preparation / gateway |
| `parallelogram` | `para`, `io` | slanted rect | input / output |
| `trapezoid` | | narrow top | manual operation |
| `cylinder` | `db`, `database` | database drum | data store |
| `card` | | cut-corner rect | file / artifact |
| `subroutine` | `sub` | double-railed rect | predefined process |
| `dblcircle` | | concentric circles | accept / final state |
| `octagon` | `stop` | stop sign | halt / terminate |
| `triangle` | | point-up triangle | extract / merge point |
| `note` | | folded-corner rect | annotation / comment |
| `tag` | | pointed-end rect | label / off-page link |

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
l "subroutine"    [subroutine]    right-of k gap 40
m "dblcircle"     [dblcircle]     below j gap 60
n "octagon"       [octagon]       right-of m gap 40
o "triangle"      [triangle]      right-of n gap 40
p "note"          [note]          below m gap 60
q "tag"           [tag]           right-of p gap 40
```

## Icons

Any node can carry an icon glyph, drawn before the first label line. Use a
named icon from the built-in set, or any literal glyph string:

```gfd
auth  "auth service" [rounded, icon=lock]
store "orders"       [db, icon=db]
fn    "resize"       [rounded, icon="λ"]
```

Named icons (`icon=NAME`): `alert api bolt book box bug build cache calendar
camera chart chat check clock cloud code config cpu cross db doc download
email event file fire flag folder gear globe heart home idea key link lock
mobile money music pin queue robot rocket search server shield star stop
sync terminal timer trash unlock upload user users warn web`.

Icons are plain text all the way through — they render identically on the
canvas and in SVG/PNG exports, and they respect the node's text color.
Classes can set them too, via the well-known `icon` variable:

```gfd
class Db(name) {
  shape = [cylinder]
  label = $name
  icon  = "db"
}
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
