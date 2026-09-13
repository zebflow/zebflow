# Web Authoring

Authoritative code and docs:

- `src/rwe/`
- `src/platform/web/templates/pages/`
- `src/platform/help/web/index.md`
- `src/platform/help/web/tailwind.md`
- `src/platform/help/web/design-system.md`
- `src/platform/help/web/libraries.md`
- `src/platform/help/web/hooks.md`
- `src/platform/help/web/ui.md`
- `blessed/source-libraries/ui/` (zeb/ui) and `blessed/rwe-libraries/` (zeb/* runtime libraries)

Contract boundary:

- `docs/developer/rwe.md`
- `docs/contracts/project.md`
- `docs/contracts/versioning.md`

Rules:

- Use Zeb React and Zeb Tailwind.
- Use `className`, not `class`.
- Use `@/` alias for component imports.
- Every file imports what it uses from `"zeb/react"`, `"zeb/ui/<name>"` or `"@/…"`; there are no injected globals and a hook used without an import is refused (`src/rwe/core/zeb_react.rs` `EXPORTS` is the list).
- Project pages use `zeb/ui/*`; the platform's own pages use `@/components/ui/*` (the studio kit). Do not mix the two on one page.
- Colours are theme roles (`bg-primary`, `text-muted-foreground`), never palette classes.
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
- `zeb/livegeo`
- `zeb/markdown`
- `zeb/pdf`
- `zeb/react` (built-in core UI hooks and rendering API, including `ErrorBoundary`
  with `fallbackRender`/`resetKeys` and `useSyncExternalStore`; external stores must
  provide a matching `getServerSnapshot` for SSR and hydration)
- `zeb/prosemirror`
- `zeb/threejs`
- `zeb/threejs-vrm`
- `zeb/use`
