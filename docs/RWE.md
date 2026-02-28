# Zebflow RWE

This document describes the shipped Reactive Web Engine contract.

## Boundary

RWE is a compile/render engine.

It should remain generic about:

1. local module resolution
2. trusted local script/style injection
3. runtime bundle mounting

It should not directly own a product library catalog.

That means `zeb/*` library policy, versioning, vendoring, and install UX belong
to the `platform` layer. RWE only needs the generic machinery required to
consume locally provided modules and assets.

## Authoring Contract

RWE is TSX-first.

There are only two semantic template roles:

1. page
2. component

Page templates use:

1. `export const page = { ... }`
2. `export const app = { ... }` (optional)
3. `export default function Page(input) { return <Page>...</Page>; }`

Component templates use:

1. `export const app = { ... }` (optional)
2. `export default function Component(props) { return (...); }`

Only page templates should be selected as render roots by a route layer or
`web_render` node.

## Page Contract

The page root is the reserved intrinsic `<Page>...</Page>`.

Example:

```tsx
import Button from "@/components/ui/button";

export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans",
  },
  navigation: "history",
};

export const app = {
  state: {
    ui: {
      advanced: false,
    },
  },
};

export default function Page(input) {
  return (
    <Page>
      <main className="max-w-5xl mx-auto px-6 py-16">
        <h1 className="text-4xl font-black tracking-tight">
          {input.hero.title}
        </h1>
        <Button type="button" label="Continue" variant="primary" size="md" />
      </main>
    </Page>
  );
}
```

Current supported page metadata:

1. `page.head.title`
2. `page.head.description`
3. `page.head.meta`
4. `page.head.links`
5. `page.head.scripts`
6. `page.html.lang`
7. `page.html.className`
8. `page.body.className`
9. `page.navigation`

## Theme And Base Style Contract

Theme and base styles are compile-scoped assets owned by the template tree.

RWE discovers stylesheet entries from the current `template_root`.

Deterministic default probe:

1. `styles/main.css`

This keeps the theme contract:

1. local to the project or platform template tree
2. git-sync friendly
3. available to the compiler without hidden global state

If explicit style entries are needed, they are supplied through
`ReactiveWebOptions.templates.style_entries`. Those entries are:

1. relative to `template_root`
2. boundary-checked under `template_root`
3. compile-time only
4. treated as strict inputs when explicitly listed

Example project structure:

```text
templates/
  pages/
    home.tsx
  components/
    ui/
      button.tsx
  styles/
    main.css
```

Example `styles/main.css`:

```css
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap");

:root {
  --zf-color-accent: #dc2626;
  --zf-font-sans: "IBM Plex Sans", system-ui, sans-serif;
  --zebflow-font-sans: var(--zf-font-sans);
}

[data-theme="docs"] {
  --zf-color-accent: #0f766e;
}
```

RWE injects discovered stylesheet entries into the rendered document before the
generated Tailwind-like utility block. This makes theme tokens and project CSS
available to the rest of the page compile.

For fonts, the current contract is:

1. load the font in discovered CSS (`@import` or `@font-face`)
2. define the token in template-owned CSS
3. expose the utility-facing variables when using `font-sans` / `font-mono`

Example:

```css
:root {
  --zf-font-sans: "IBM Plex Sans", system-ui, sans-serif;
  --zf-font-mono: "IBM Plex Mono", monospace;
  --zebflow-font-sans: var(--zf-font-sans);
  --zebflow-font-mono: var(--zf-font-mono);
}
```

RWE uses this contract to generate the full document shell:

1. `<html ...>`
2. `<head>...</head>`
3. `<body ...>...</body>`

That means page templates should not manually author `<html>`, `<head>`, or
`<body>` in the canonical path.

Additional head entries are declared structurally:

```tsx
export const page = {
  head: {
    title: "{{input.seo.title}}",
    description: "{{input.seo.description}}",
    links: [
      { rel: "canonical", href: "{{input.seo.canonical}}" }
    ],
    meta: [
      { property: "og:title", content: "{{input.seo.title}}" }
    ],
    scripts: [
      { src: "https://unpkg.com/lucide@0.469.0/dist/umd/lucide.min.js" }
    ]
  }
};
```

## Import Contract

Import resolution is compile-scoped, not global.

Important compile-time inputs:

1. `ReactiveWebOptions.templates.template_root`
2. `ReactiveWebOptions.templates.style_entries`
3. `TemplateSource.source_path`

Supported imports:

1. `@/components/...`
2. `./...`
3. `../...`

Current limits:

1. compile-time only
2. boundary-checked under `template_root`
3. local `.tsx` modules only
4. no npm imports
5. no dynamic import
6. no `.ts` helper-module graph yet

This allows one RWE engine instance to safely compile:

1. Zebflow platform templates
2. different user-project template trees
3. future external consumers

without hidden global filesystem state.

## Zeb Libraries Boundary

When project code imports `zeb/*`, the product policy should be:

1. platform resolves the import against project/vendor state
2. project pins exact versions in `app/libraries.lock.json`
3. vendored copies live in `app/libraries/`
4. RWE consumes the resolved local module/assets through generic hooks

RWE should not become the owner of:

1. library catalog policy
2. install/update/remove flows
3. remote registry synchronization

## Component Contract

Normal PascalCase tags are ordinary imported components:

```tsx
import Sidebar from "@/components/sidebar";
import Button from "@/components/ui/button";

export default function Page(input) {
  return (
    <Page>
      <Sidebar />
      <Button label="Save" />
    </Page>
  );
}
```

Imported components are resolved through the import graph, then lowered and
expanded during compile. The compile-time registry is internal machinery, not
the primary user contract.

## Hydration Contract

Hydration is component-level and opt-in.

Supported modes:

1. `off` (default when omitted)
2. `interaction`
3. `visible`
4. `idle`
5. `immediate`

Example:

```tsx
<Gallery items="{{input.gallery}}" hydrate="visible" />
<CommentBox postId="{{input.post.id}}" hydrate="idle" />
<Editor hydrate="interaction" />
<LiveChart hydrate="immediate" />
```

Meaning:

1. `off`
   - server-render only
2. `interaction`
   - activate on first user interaction
3. `visible`
   - activate when entering viewport
4. `idle`
   - activate when browser is idle
5. `immediate`
   - activate on load

## Navigation Policy

For Zebflow web delivery, the canonical direction is:

1. SSR first
2. `history` as the default navigation mode
3. `document` as the explicit fallback mode

`page.navigation` is the single page-level place where that intent is declared.

## Tailwind-Like Processing

RWE includes a Tailwind-like processor pipeline.

Current behavior:

1. compile static utility classes into CSS
2. support `tw-variants` dynamic contract hints
3. warn when dynamic class placeholders do not declare their dynamic contract
4. inject compile-scoped project CSS from `template_root/styles/main.css`
5. inject generated Tailwind-like utility CSS

The key rule is:

1. static classes compile once
2. dynamic classes must be explicit and constrained
3. wildcard dynamic patterns imply runtime styling support

## Not Final Yet

These areas remain intentionally unfinished:

1. `.ts` helper-module imports
2. full page metadata surface beyond the current fields
3. editor integration contract
4. static export command family
5. named variant grammar for class macros
