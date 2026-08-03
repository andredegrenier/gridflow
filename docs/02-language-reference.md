# GFD Language Reference

This is the complete reference for the gridflow diagram language. GFD is
line-oriented: one statement per line (or separated by `;`). `//` starts a
comment that runs to the end of the line. `#` always introduces a hex color,
never a comment.

## File structure

A file is a sequence of statements. `use` statements should come first;
everything else may appear in any order (edges can reference nodes declared
later).

```
file        := (statement | comment | blank)*
statement   := use | dir | default | assignment | class | node | edge | group
```

## Identifiers, numbers, strings, colors

- **Identifiers**: `[A-Za-z_][A-Za-z0-9_]*` — node ids, variable names, class
  names, port names. No hyphens (so `a--b` can never be confused with the `--`
  edge).
- **Numbers**: integers or decimals, optionally negative: `42`, `-10`, `3.5`.
- **Strings**: double-quoted with escapes `\"`, `\\`, `\n`, `\t`. `$name`
  inside a string interpolates a variable: `"Deploy to $env"`.
- **Colors**: `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa` — e.g. `#f00`,
  `#ff6b35`, `#ff6b35cc`.

## `dir` — layout direction

```gfd
dir: TB    // top-to-bottom flow (default)
dir: LR    // left-to-right flow
```

At file scope it sets the direction for the whole diagram; inside a group it
overrides the direction for that group's members only.

## `node` — declaring nodes

```gfd
node id                      // minimal: label defaults to the id
node id "Label text"
id "Label"                   // the `node` keyword is optional
id "Label" [rounded, fill=#e3f2fd, w=180]
id$bundle "Label"            // apply ONE attribute-bundle variable
id "Label" @ (320, 260)      // pinned placement
id "Label" right-of other gap 40
```

Order of parts (all optional except the id):
`node? id ($var)? ("label")? ([attrs])? (placement)?`

### Attributes

Inside `[...]`, comma-separated:

| Attribute | Effect |
|---|---|
| shape keyword | see the **Shapes** chapter: `rect`, `rounded`, `stadium`, `circle`, `ellipse`, `diamond`, `hexagon`, `parallelogram`/`para`, `trapezoid`, `cylinder`/`db`, `card` |
| `dashed` | dashed border |
| `bold` | thick border + bold-ish label |
| `fill=<color>` | background color |
| `stroke=<color>` | border color |
| `text=<color>` | label color |
| `w=<num>` / `h=<num>` | fixed width/height (world units) |
| `$var` | splice a bundle variable's items in place |

Later items win: `[fill=#f00, fill=#0f0]` is green.

## `edge` — connecting nodes

```gfd
a -> b                       // directed
a <- b                       // directed, pointing left (b stays the target of nothing; a is the target)
a <-> b                      // bidirectional
a -- b                       // undirected
a ..> b                      // dotted directed
a <.. b                      // dotted, pointing left
a -> b : "label"             // label
a -> b : "label" [dashed, bold, stroke=#c00]
a -> b -> c -> d             // chain: three edges; label/attrs apply to each
n.out -> m.in                // port-anchored (see Classes)
```

An edge may reference an id that is never declared — gridflow renders a dashed
**phantom node** so half-typed diagrams stay visible. Declaring the node later
(or dragging the phantom, which writes a declaration) makes it real.

## `default` — baseline styles

```gfd
default node [rounded, fill=#f4f6fa]
default edge [stroke=#8a94a6]
```

Applies to every node/edge declared **after** the statement (file order).
Explicit attributes and applied `$bundles` override the defaults. A second
`default node [...]` replaces the first.

## Assignments — variables

```gfd
warn = #ff6b35               // color
gapX = 180                   // number
env  = "prod"                // string
hot  = [rounded, fill=$warn] // attribute bundle
```

Variables are **single-assignment**: reusing a name is an error. They are
dynamically typed — what you can do with `$x` depends on what `x` holds:

- `fill=$warn` — value substitution in an attribute
- `node$hot` — apply a bundle to a node
- `"[$env] api"` — string interpolation
- `[$hot, bold]` — splice a bundle into another bundle (last item wins)

A node may apply **at most one** `$bundle` (`n$a$b` is an error) — compose
bundles at definition time instead: `c = [$a, $b]`.

## `class` — node archetypes

```gfd
class Svc(name, color=#eef2f7) {
  shape = [rounded]
  label = $name
  fill  = $color
  port in  left
  port out right
}

api = Svc("API")
db  = Svc("Postgres", #ede7f6) below api
api.out -> db.in
```

See the **Variables & Classes** chapter for full semantics. Summary:

- Parameters are positional; trailing parameters may declare defaults.
- The class body is a list of variable assignments and `port` declarations.
- Well-known variables map onto the node: `shape` (a bundle), `label`, `fill`,
  `stroke`, `w`, `h`. Any other variables are just intermediates.
- `port <name> <side>` declares a named attach point (`top`/`bottom`/`left`/
  `right`). Multiple ports on one side space themselves evenly.
- Instantiation is `id = ClassName(args...)` with an optional placement.
  Instances are pure stamps — no per-instance overrides.

## `group` — visual containers

```gfd
group deploy "Deployment" @ (200, 520) {
  dir: LR
  node stage "Staging"
  node prod  "Prod" right-of stage gap 60
  stage -> prod
}
```

Groups draw a labeled frame around their members and lay them out as a
cluster (with an optional local `dir`). A group can be pinned (`@`), placed
relative to a node, or left to auto-layout (the cluster then participates in
the top-level layout as one big node). Dragging a group's **title bar** moves
the whole cluster. Groups do not nest.

## `use` — libraries

```gfd
use flow, people             // from the global library directory
use "./team-styles.gfd"      // relative to this document's folder
```

Imports the variables and classes (only) from the named files. Document-local
definitions shadow imports; two imports defining the same name is an error.
See the **Libraries** chapter.

## Placement clauses

| Clause | Meaning |
|---|---|
| *(none)* | auto-layout |
| `@ (x, y)` | pinned at world coordinates (node center) |
| `right-of ID gap N` | N world-units to the right of ID (default gap 50) |
| `left-of ID`, `above ID`, `below ID` | the other directions |

Relative placements resolve against the anchor's *final* position, so hints
chain (`c right-of b`, `b right-of a`). A cycle of hints is a diagnostic and
the nodes fall back to auto-layout.

## Error recovery

The parser recovers per statement: a syntax error underlines that statement
and skips to the next line. The resolver degrades the same way (an unknown
`$var` or class leaves the node with default styling plus a diagnostic).
The canvas is never blanked by an error.

## Formal grammar (EBNF-ish)

```
statement    := use | dir | default | assign | class | node_decl | edge_decl | group_decl
use          := "use" use_item ("," use_item)*
use_item     := IDENT | STRING
dir          := "dir" ":" ("TB" | "LR")
default      := "default" ("node" | "edge") attrs
assign       := IDENT "=" (value | call) placement?
call         := IDENT "(" (value ("," value)*)? ")"
class        := "class" IDENT params? "{" (class_var | port)* "}"
params       := "(" param ("," param)* ")"
param        := IDENT ("=" value)?
class_var    := IDENT "=" value
port         := "port" IDENT ("top"|"bottom"|"left"|"right")
node_decl    := "node"? IDENT ("$" IDENT)? STRING? attrs? placement?
edge_decl    := endpoint (arrow endpoint)+ (":" STRING)? attrs?
endpoint     := IDENT ("." IDENT)?
arrow        := "->" | "<-" | "<->" | "--" | "..>" | "<.."
group_decl   := "group" IDENT STRING? placement? "{" (node_decl|edge_decl|dir)* "}"
attrs        := "[" (attr ("," attr)*)? "]"
attr         := SHAPE_KW | "dashed" | "bold"
              | ("fill"|"stroke"|"text"|"w"|"h") "=" value
              | "$" IDENT
value        := COLOR | NUMBER | STRING | "$" IDENT | attrs
placement    := "@" "(" NUMBER "," NUMBER ")"
              | ("right-of"|"left-of"|"above"|"below") IDENT ("gap" NUMBER)?
```
