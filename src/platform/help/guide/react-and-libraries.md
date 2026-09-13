# React + Internal Libraries

Zebflow web UI uses React-style TSX through the Reactive Web Engine.

## Main rule

Use Zebflow's own frontend surface:

- Zeb React hooks
- Zeb Tailwind styling
- `zeb/*` libraries

## Why

This keeps:

- rendering model consistent
- bundle/runtime rules consistent
- server-render + client-hydration behavior predictable

## Internal library families

Two surfaces need no install: `zeb/react` (hooks, in every file) and `zeb/ui`
(components, `zeb/ui/<name>`). Beyond those, the runtime library set is:

- `zeb/d3`, `zeb/deckgl`, `zeb/codemirror`, `zeb/markdown`, `zeb/pdf`,
  `zeb/prosemirror`, `zeb/threejs`, `zeb/threejs-vrm`, `zeb/graphui`,
  `zeb/livegeo`, `zeb/use`

Each is imported as a static `import { … } from "zeb/<lib>"` after it is
enabled for the project; a dynamic `import()` of one is refused by default.
There is no icon library, and no Preact — the render model is React-style TSX
through the RWE only.

These are meant to be the first-class frontend layer inside Zebflow projects.

For detailed rules, see:

- `help("web")`
- `help("web/libraries")`
