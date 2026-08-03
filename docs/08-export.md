# Export

File menu → **Export SVG…** or **Export PNG (2x)…**

## SVG

A self-contained `.svg` with no external dependencies:

- Same geometry as the canvas (both are generated from one Scene model).
- Text as real `<text>` elements (searchable, editable in Inkscape/Figma).
- Light, print-friendly palette regardless of your app theme.

SVG is the right choice for embedding in docs, wikis, and READMEs — it scales
forever and diffs reasonably in git.

## PNG

The PNG exporter rasterizes gridflow's own SVG output (via resvg) at 2×
resolution, so PNG and SVG always match pixel-for-pixel modulo scaling. Use
PNG for chat apps and slides that don't take SVG.

## Tips

- The export bounds are the diagram's bounding box plus a margin — no need to
  frame anything first.
- Multi-line labels, port-anchored edges, dashed styles, and group frames all
  export.
- Want a different look in exports (dark theme, custom fonts)? That's planned;
  today exports use the standard light style.

## Diagram-as-code in a repo

A pleasant workflow for keeping architecture diagrams in a project:

```
repo/
  docs/
    architecture.gfd          ← source of truth (edit in gridflow)
    architecture.notes.md     ← the prose that goes with it
    architecture.svg          ← exported, committed for viewing on GitHub
    team-styles.gfd           ← shared look, imported with use "./team-styles.gfd"
```

Re-export after editing and the PR shows both the text diff (reviewable!) and
the rendered image.
