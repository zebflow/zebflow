# Web Authoring

Authoritative code and docs:

- `src/rwe/`
- `src/platform/web/templates/pages/`
- `src/platform/help/web/index.md`
- `src/platform/help/web/tailwind.md`
- `src/platform/help/web/design-system.md`
- `src/platform/help/web/libraries.md`
- `src/platform/help/web/hooks.md`
- `libraries/`

User-facing docs:

- `docs/usage/templates.md`
- `docs/usage/project-studio.md`

Rules:

- Use Zeb React and Zeb Tailwind.
- Use `className`, not `class`.
- Use `@/` alias for component imports.
- Do not import Preact hooks from npm paths.
- Entry pages may import from `zeb` for editor hints; component files should rely on injected globals.
- Behavior `.ts` exports should be camelCase, not ALL_CAPS.
- Fix shared RWE/Tailwind/library behavior when the same issue appears in multiple pages.

Project Studio surfaces:

- home
- hub
- login
- profile
- dashboard
- pipelines
- registry/editor
- files
- credentials
- DB connections
- infrastructure
- settings

Library surfaces:

- `zeb/codemirror`
- `zeb/d3`
- `zeb/deckgl`
- `zeb/graphui`
- `zeb/icons`
- `zeb/livegeo`
- `zeb/markdown`
- `zeb/pdf`
- `zeb/preact`
- `zeb/prosemirror`
- `zeb/threejs`
- `zeb/threejs-vrm`
- `zeb/use`
