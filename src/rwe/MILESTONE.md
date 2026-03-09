# RWE Milestone

1. lightspeed tsx compilation
   - Same principle like before, compile only on commit, cached render
2. ergonomic tsx editor
3. lightweight client-first completion
   - Standard completion for general syntax and types
   - Completion for imported components
   - Should we create our limited scope of completion?
      - Lightweight completion reference
4. independent smooth components modularity
   - The experience of creating modular deep component need to be as enjoyable or more enjoyable than general React dev
5. go to definition
6. highly sandboxed templates

Folder structure
- data
- files
- git synced folder
  - pipelines
  - assets
  - templates

## TSX and TS Specs

------------- Forget the text after this ------------

This document tracks the RWE roadmap with one priority:

- keep runtime lean
- keep authoring simple
- keep SSR/SEO first-class

## Target Outcome

RWE should support:

1. modular `.tsx` pages and components
2. strong server/client separation
3. fast on-the-fly compile/render in `dev`
4. production `build` outputs for `ssr` and `spa`
5. predictable hydration cost

## Current Baseline (Done)

1. `.tsx` extraction (`template`, `style`, `script`)
2. reactive bindings via `export const app` (state/actions/memo/effect) and `usePageState()` hook
3. SSR placeholders (`{{input.*}}`, `{{ctx.*}}`)
4. compile-time component registry (PascalCase component tags, e.g. `<PlatformSidebar />`)
5. runtime bundle inject + control script mount

## Canonical Runtime Strategy

For Zebweb, the canonical web-delivery model is:

1. server-side render first
2. client-history navigation by default
3. selective hydration islands per component
4. `document` navigation remains available as an explicit fallback mode

Meaning:

1. direct URL access always resolves on server/webhook
2. the first response is SEO-friendly HTML
3. later route transitions may stay inside one browser document
4. only components that need interactivity pay hydration/runtime cost

This is not a pure SPA and not a document-only site.
It is an SSR-first app shell with SPA navigation capability.

Static export is a separate concern:

1. do not treat Zebweb as a `build`-first workflow
2. dev/on-the-fly authoring is the primary runtime story
3. static site output should be introduced later under a separate command family
   so users do not confuse static export with the canonical SSR app runtime

## Route Contract Direction

Keep two independent axes:

1. render strategy
   - `ssr`
   - `ssg` later
   - `client` later
2. navigation strategy
   - `history` default
   - `document` optional

For current Zebweb work, default page contract should effectively mean:

```ts
export const page = {
  render: "ssr",
  navigation: "history",
};
```

Then component-level hydration decides which parts become interactive.

## Component Hydration Direction

Default rule:

1. components are server-rendered by default
2. components only hydrate when explicitly marked

Initial hydration modes:

1. `off`
2. `interaction`
3. `visible`
4. `idle`
5. `immediate`

This keeps neutral/SEO components cheap by default.

## Milestone Plan

## M1 - File-Based Modularity

Goal: stop relying only on in-memory component registry.

Deliverables:

1. file component reference syntax:
   - `import { Searchbar } from "./searchbar.tsx";`
   - `import { Sidebar } from "../shared/sidebar.tsx";`
2. resolver with project-root boundary check
3. compile dependency graph (page -> components -> nested components)
4. hash cache per source file

Why:

- lets `sidebar.tsx` include `searchbar.tsx` directly
- fits real folder workflows

## M2 - Server/Client State Contract

Goal: make mixed SSR + interactivity explicit and safe.

Proposed script contract:

1. `server` namespace:
   - `load(input, ctx, n)` returns SSR data
   - optional `expose` list to control hydration payload
2. `client` namespace:
   - `state`, `actions`, `memo`, `effect`
   - only mutable on browser

Minimal shape:

```js
return {
  server: {
    async load(input, ctx, n) { return { posts: [] }; },
    expose: ["posts", "seo"]
  },
  client: {
    state: { filter: "" },
    actions: { "filter.set": (ctx, payload) => { ... } },
    memo: { ... },
    effect: { ... }
  }
};
```

Why:

- removes ambiguity between server and client data
- makes hydration payload intentionally small

## M3 - SSR List/Branch Primitives

Goal: lean server list rendering via standard TSX.

Use standard JSX patterns:

1. `{(input.posts || []).map((item) => <li key={item.id}>...</li>)}`
2. `{condition && <element />}`
3. `{condition ? <a /> : <b />}`

Why:

- standard React/Preact — no custom directives
- supports blog list, dashboard table, catalog pages
- SEO-friendly by default

## M4 - Hydration Boundaries

Goal: pay JS cost only where needed.

Per-block strategy:

1. `hydrate="off"` for pure SSR block
2. `hydrate="visible"` lazy mount
3. `hydrate="interaction"` mount on first user interaction
4. `hydrate="idle"` mount on browser idle

Why:

- keeps pages fast while still interactive

## M5 - Build Output Modes

Goal: clean production outputs for app/deploy workflows.

1. `build --ssr`:
   - route manifests
   - server render artifacts
2. `build --spa`:
   - client-only bundle
3. `build` default:
   - hybrid output with static assets + route metadata

## Folder Convention (Proposed)

```txt
app/
  pages/
    blog-home.tsx
    blog-post.tsx
  components/
    sidebar.tsx
    sidebar/
      searchbar.tsx
  layouts/
    main.tsx
```

Rules:

1. pages are entrypoints
2. components can nest freely
3. only pages define route metadata

## Non-Goals (For Now)

1. full React compatibility layer
2. generic VDOM reconciler
3. broad plugin API before core SSR/hydration model stabilizes

## Immediate Next Tasks

1. implement file import resolver with cycle detection
2. add component dependency graph cache
3. implement `server/client` script contract
4. add fixture coverage for SSR list rendering via `.map()`
5. add fixture set:
   - `sidebar.tsx` + nested `searchbar.tsx`
   - SSR blog list from server data
