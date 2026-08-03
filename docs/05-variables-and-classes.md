# Variables & Classes

## Variables

A variable binds a name to a value with `name = value`. Values are dynamically
typed — the four kinds:

```gfd
warn  = #ff6b35                  // color
gapX  = 180                      // number
env   = "production"             // string
hot   = [rounded, fill=$warn]    // attribute bundle
```

Rules:

- **Single assignment.** A name is bound once per file; rebinding is an
  error. (Shadowing a library import with a local definition is allowed and
  silent.)
- **Use-before-define is an error** — order your file top-down.
- Variables can reference earlier variables (`hot` uses `$warn` above).

### The three uses of `$`

1. **Value substitution** — anywhere a value is expected:
   ```gfd
   n [fill=$warn, w=$gapX]
   b right-of a gap $gapX      // (numbers in placements: coming soon — today
                               //  placements take literal numbers)
   ```
   *(Placement gaps currently require literal numbers; `fill=$warn` and
   `w=$gapX` work today.)*

2. **Bundle application** — suffix on a node id:
   ```gfd
   server$hot "API server"
   ```
   Applies every item in the bundle as if written inline. **One `$` per
   node** — to combine looks, compose a new bundle:
   ```gfd
   critical = [$hot, bold, stroke=#c62828]
   server$critical "API server"
   ```

3. **String interpolation** —
   ```gfd
   title "Deploys to $env"     // renders: Deploys to production
   ```

Precedence on a node, weakest to strongest:
`default node [...]` → applied `$bundle` → inline `[...]` attributes.
Within any list, later items beat earlier ones.

## Classes

A class is a reusable **single-node archetype**: a bundle of variables plus
named ports, stamped out by instantiation.

```gfd
class Svc(name, color=#eef2f7) {
  shape  = [rounded]
  label  = $name
  fill   = $color
  port in  left
  port out right
}

auth = Svc("Auth")                    // uses the default color
pay  = Svc("Payments", #ffe0b2)       // positional override
pay2 = Svc("Payments EU", #ffe0b2) below pay gap 40
```

### Parameters

- Positional; listed defaults make trailing parameters optional.
- A missing required argument, an extra argument, or a type mismatch is a
  diagnostic — the node still renders with whatever resolved.
- Arguments are evaluated in the *caller's* scope, so you can pass `$vars`.

### Class body

- Assignments run top-to-bottom; each sees the parameters, earlier class
  variables, then the caller's scope (so classes can use file/library
  variables like a shared palette).
- **Well-known names** map onto the produced node:

  | Variable | Type | Effect |
  |---|---|---|
  | `shape` | bundle | applied like inline attrs (shape + any styling) |
  | `label` | string | node label |
  | `fill`, `stroke` | color | colors |
  | `w`, `h` | number | fixed size |

  Anything else is an intermediate value.

- **Ports**: `port <name> <side>` where side is `top`/`bottom`/`left`/`right`.
  Ports on the same side distribute evenly along it. Edges attach with
  dot-syntax:

  ```gfd
  auth.out -> pay.in : "invoices"
  ```

  Referencing a port the node doesn't declare is a diagnostic (the edge falls
  back to boundary anchoring).

### Instances are stamps

There are deliberately **no per-instance overrides** — an instance is exactly
what the class says (plus placement). If you need a variant, make a variant
class or add a parameter. This keeps every instance honest: reading the class
tells you everything about all of its instances.

### Classes vs bundles — which to use?

| | Bundle (`x = [...]`) | Class |
|---|---|---|
| Reusable styling | ✔ | ✔ |
| Label templating (`$name`) | ✘ | ✔ |
| Ports | ✘ | ✔ |
| Parameters | ✘ | ✔ |
| Applied to an existing node | ✔ (`n$x`) | ✘ (creates the node) |

Rule of thumb: bundles style nodes you declare by hand; classes *are* the
node, for repeated structures (services, people, queues).
