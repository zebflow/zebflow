# Template Formal Guide

This guide defines the intended **stable mental model** for the Zebflow Template.

Zebflow Template is based on the **Reactive Web Engine (RWE)**.

- It ships React-like TSX.
- It includes a built-in Tailwind-like renderer.
- It compiles and renders inside Zebflow without a separate frontend build step.
- It is supported by:
  - `zeb`
  - `zeb/*`
  - `Tool.*`

## 1. First Rule

A page template is a **TSX module** whose page entry is:

```tsx
export default function Page(input) {
  return <div />;
}
```

`Page(input)` is the **render entry**.

- `input` is the immutable readable input from pipeline's n.web.response and RWE configuration.
- `Page(input)` is for rendering only.
- `Page(input)` does not set headers, status, cookies, or page config.

## 2. Page Entry

`Page(input)` is the render entry for a full page/document.

- `input` is the canonical page input object.
- A page is recognized by **entry semantics**, not by folder name.

Example:

```tsx
export default function Page(input) {
  return <h1>{input.title}</h1>;
}
```

## 3. `page`

`export const page = { ... }` defines static page/document config.

Use it for:

- title
- head metadata
- html/body attributes

Example:

```tsx
export const page = {
  title: "Home",
  html: { lang: "en" },
};
```

`page` is for document config only.

- It does not render body content.
- It does not mutate response state.

## 4. `getPage(input)`

`export function getPage(input) { ... }` defines dynamic page/document config.

Use it when page config must be computed from `input`.

Example:

```tsx
export function getPage(input) {
  return {
    title: input.post?.title ?? "Untitled",
  };
}
```

`getPage(input)` is for document config only.

- It does not render body content.
- It does not perform backend orchestration.

## 5. Components

A component is a reusable imported TSX module.

- component = imported module
- page = entry module

Example:

```tsx
export function UserCard({ user }) {
  return <div>{user.name}</div>;
}
```

## 6. Reusables

Templates can import reusable modules from the template root.

- TSX components
- `.ts` helper/script modules

`@/` means the template root.

Examples:

```tsx
import { UserCard } from "@/components/user-card";
import { formatDate } from "@/scripts/date";
```

Relative imports are also allowed when appropriate.

Examples:

```tsx
import { UserCard } from "./components/user-card";
import { formatDate } from "../scripts/date";
```

## 7. Imports

Official template imports are:

- `zeb`
- `zeb/*`
- `@/`

Example:

```tsx
import { usePageState } from "zeb";
import { Button } from "@/shared/ui/button";
```

## 8. Boundary

There is one strict split:

- `Page(input)` reads and renders.
- `page` and `getPage(input)` describe document config.

Template code does not own backend orchestration.

## 9. `input`

`input` is the immutable readable page input object passed into:

```tsx
export default function Page(input) { ... }
export function getPage(input) { ... }
```

In current Zebflow page rendering through `n.web.response`, `input` is:

```ts
{
  ...upstreamPayload,
  params?,
  query?,
  headers?,
  auth?
}
```

Meaning:

- `...upstreamPayload` is the payload from the last upstream pipeline node
- `params` is injected from the trigger when absent
- `query` is injected from the trigger when absent
- `headers` is injected from the trigger when absent
- `auth` is injected from the trigger and filtered to public claims only

Example:

```tsx
export default function Page(input) {
  return (
    <article>
      <h1>{input.post?.title ?? input.title}</h1>
      <p>{input.params?.slug}</p>
      <p>{input.query?.preview ? "Preview" : "Published"}</p>
    </article>
  );
}
```

`input` is readable input only.

- It is not a response writer.
- It does not set headers, cookies, or status.
- It does not replace backend orchestration.

## 10. CSS

Templates write normal class-based TSX markup.

Example:

```tsx
export default function Page(input) {
  return (
    <div className="min-h-screen bg-stone-950 text-stone-100">
      <h1 className="text-4xl font-bold tracking-tight">{input.title}</h1>
    </div>
  );
}
```

## 11. Tailwind Engine

Zebflow templates use the built-in Tailwind-like renderer from RWE.

- Utility classes are compiled inside Zebflow.
- Rendered CSS is produced by the engine at compile/render time.
- Templates do not require a separate frontend build pipeline.

Example:

```tsx
export default function Page() {
  return (
    <section className="mx-auto max-w-3xl px-6 py-12">
      <div className="rounded-3xl border border-stone-800 bg-stone-900/70 p-8 shadow-2xl">
        Hello
      </div>
    </section>
  );
}
```

## 12. Formal Styling Guide

Zebflow Template uses a **React-compatible layered styling model**.

You can style templates with:

- `className`
- `style={{ ... }}`
- side-effect CSS imports
- raw `<style>` blocks

Zebflow adds:

- the built-in Tailwind-like engine
- `styles/main.css`
- `tw-variants`
- document-shell styling through `page`

### 12.1 Primary Methods

Use these first:

1. utility classes in TSX
2. `styles/main.css`
3. `tw-variants` for dynamic utility-class cases

### 12.2 Utility Classes in TSX

This is the primary styling method.

Example:

```tsx
export default function Page(x) {
  return (
    <section className="mx-auto max-w-4xl px-6 py-12">
      <div className="rounded-3xl border border-stone-800 bg-stone-900/70 p-8 shadow-2xl">
        <h1 className="text-4xl font-bold tracking-tight">{x.title}</h1>
      </div>
    </section>
  );
}
```

### 12.3 Global Project CSS in `styles/main.css`

Use `styles/main.css` for project-level styling foundation.

Use it for:

- CSS variables
- fonts
- semantic helper classes
- media queries
- print rules
- scrollbar styling
- long animations
- selectors utility classes express poorly
- semantic color tokens for the Tailwind-like engine

Example:

```css
:root {
  --zf-accent: #38bdf8;
}

.zf-shell {
  max-width: 72rem;
  margin: 0 auto;
}
```

```tsx
export default function Page() {
  return <div className="zf-shell">Hello</div>;
}
```

The Tailwind-like engine also reads semantic color tokens from `main.css`.

If `main.css` defines:

```css
:root {
  --color-brand-blue: #005b9a;
  --color-surface: #111827;
  --color-body-soft: #6b7280;
}
```

then TSX can use:

```tsx
<h1 className="text-brand-blue">Hello</h1>
<div className="bg-surface text-body-soft border-brand-blue">...</div>
```

Semantic token rules:

- use lowercase names only
- use hyphenated names only
- define tokens as `--color-*`
- use semantic names, not Tailwind-like palette scales

Prefer:

- `brand-blue`
- `surface`
- `surface-2`
- `body`
- `body-soft`
- `border`
- `border-soft`

Avoid:

- camelCase names
- custom palette-like names such as `blue-500` or `blue-sky-500`

### 12.4 Dynamic Utility Classes with `tw-variants`

Use `tw-variants` when utility classes are dynamic and the compiler needs explicit hints.

Example:

```tsx
<span
  hidden
  tw-variants="bg-green-900/40 text-green-300 bg-amber-900/40 text-amber-300 bg-red-900/40 text-red-300"
/>
```

### 12.5 Document-Shell Styling with `page`

Use `page` for document-level styling, especially:

- `page.body.className`

Example:

```tsx
export const page = {
  body: {
    className: "min-h-screen bg-stone-950 text-stone-100",
  },
};
```

### 12.6 Extra Stylesheets

Extra stylesheet files may be used when needed.

Use this for:

- library CSS
- page or feature CSS that should stay outside utility markup

This is secondary to `styles/main.css`.

Example:

```tsx
import "@/styles/editor.css";
```

### 12.7 Inline `style={{ ... }}`

Inline style is allowed for **dynamic geometry or measured layout values**.

Use it for:

- width
- height
- transform
- coordinates
- CSS variables when computed dynamically

Example:

```tsx
<div style={{ width: `${x.progress}%` }} />
```

Do not use inline style as the primary styling system.

### 12.8 Raw `<style>` Blocks

Raw `<style>` blocks are a local escape hatch.

Use them for:

- page-local CSS
- one-off keyframes
- small isolated rules

Example:

```tsx
export default function Page() {
  return (
    <>
      <style>{`
        .pulse-dot {
          animation: pulse 1.4s infinite;
        }
        @keyframes pulse {
          0%, 100% { opacity: 0.35; }
          50% { opacity: 1; }
        }
      `}</style>
      <div className="pulse-dot">•</div>
    </>
  );
}
```

Use this sparingly. It is not the primary shared styling path.

### 12.9 Preferred Order

The preferred order is:

1. utility classes in TSX
2. `styles/main.css`
3. `tw-variants`
4. inline `style={{ ... }}`
5. raw `<style>` blocks

## 13. Flow

```text
+---------------------------+
| TSX Template Module       |
|                           |
| export default Page(input)|
| export const page         |
| export function getPage   |
|                           |
| happens in: repo template |
| file authored by user     |
| engine: TSX source module |
+---------------------------+
            |
            v
+---------------------------+
| Compile Phase             |
|                           |
| parse TSX                 |
| validate imports          |
| bundle runtime            |
| build compiled template   |
|                           |
| happens in: RWE compiler  |
| on backend/server side    |
| engine: Rust              |
+---------------------------+
            |
            v
+---------------------------+
| SSR Render Phase          |
|                           |
| backend passes render     |
| payload into RWE          |
|                           |
| happens in: n.web.response|
| and RWE render boundary   |
| engine: Rust / Axum-like  |
| web response boundary     |
+---------------------------+
            |
            v
+---------------------------+
| JS Runtime Injection      |
|                           |
| globalThis.ctx = payload  |
| globalThis.input = ctx    |
|                           |
| happens in: SSR JS runtime|
| before module load        |
| engine: Deno / V8         |
+---------------------------+
            |
            v
+---------------------------+
| Module Evaluation         |
|                           |
| page = static config      |
| getPage(input)=dynamic cfg|
| Page(input)=render body   |
|                           |
| happens in: SSR JS runtime|
| during module execution   |
| engine: Deno / V8         |
+---------------------------+
            |
            v
+---------------------------+
| HTML Output               |
|                           |
| document shell            |
| SSR HTML                  |
| hydration payload         |
|                           |
| happens in: backend/server|
| response generation       |
| engine: Rust              |
+---------------------------+
            |
            v
+---------------------------+
| Browser Hydration         |
|                           |
| client receives payload   |
| client sets ctx again     |
| Page(input) hydrates      |
|                           |
| happens in: browser/client|
| after HTML is delivered   |
| engine: Browser JS engine |
+---------------------------+
```
