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

Examples:

- `zeb/use`
- `zeb/deckgl`
- `zeb/pdf`
- `zeb/markdown`
- `zeb/icons`

These are meant to be the first-class frontend layer inside Zebflow projects.

For detailed rules, see:

- `help("web")`
- `help("web/libraries")`
