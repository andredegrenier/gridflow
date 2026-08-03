# Libraries

Libraries let you share variables and classes across diagrams. A library file
is just a `.gfd` file — only its **assignments and classes** are imported;
any nodes/edges in it are ignored.

## The global library

Lives in your per-user config directory:

- macOS: `~/Library/Application Support/gridflow/lib/`
- Windows: `%APPDATA%\gridflow\config\lib\`
- Linux: `~/.config/gridflow/lib/`

Every `.gfd` file there is importable by bare name:

```gfd
use flow            // loads <lib>/flow.gfd
use flow, people    // multiple imports
```

On first launch gridflow seeds the library with three starters (never
overwriting your files):

| File | Contents |
|---|---|
| `flow` | palette variables, style bundles (`primary`, `danger`, …) and classes `Process`, `Decision`, `Terminal`, `Data`, `Store` |
| `people` | `Person(name, role)` and `Team(name)` for org charts |
| `aws` | `Service`, `Lambda`, `Queue`, `Db`, `Bucket`, `Gateway` |

Try it:

```gfd
use flow
in   = Data("Order JSON")
val  = Process("Validate")
ok   = Decision("Valid?")
save = Store("orders db")
in -> val -> ok
ok.yes -> save.in
```

## Document-relative libraries

For libraries that should travel with a project (e.g. in a git repo), use a
quoted path — resolved relative to the document's folder:

```gfd
use "./team-styles.gfd"
use "../shared/palette.gfd"
```

A repo can then carry its house style alongside its diagrams, and everything
works on any machine after a plain `git clone`.

## Name resolution

1. Definitions in the current document win (they shadow imports silently).
2. Otherwise the name comes from whichever `use` provided it.
3. If two different `use`d files define the same name → diagnostic; resolve
   it by removing one import or renaming in your copy of the library.

## The library pane (⌘3)

- Lists every file in the global library.
- **use** button inserts the `use` line at the top of your document.
- **Save class to library**: every class defined in the current document is
  listed; pick a target file name and click save — the class's source text is
  appended to `<target>.gfd` in the global library. That's also the easiest
  way to grow your collection: prototype a class in a diagram, then promote it.

## Authoring tips

- Keep a palette file (`colors.gfd`) with nothing but color variables, and
  build style libraries on top of it (`use` doesn't chain — a library file's
  own `use` lines are currently ignored, so keep libraries self-contained).
- Library diagnostics (a broken class in a library) surface in the importing
  document at the `use` line.
- Libraries are plain text: check them into a dotfiles repo, share them in a
  gist, or sync the folder with any file-sync tool. (Team-synced libraries
  arrive with the collaboration phase.)
