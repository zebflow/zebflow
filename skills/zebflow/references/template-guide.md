# Zebflow Template Guide — Full Reference

Templates are TSX modules compiled and rendered inside Zebflow using OXC (parser/transpiler) and an embedded V8 worker (deno_core). No external build step.

---

## Page Entry

Every page has one render entry:

```tsx
export default function Page(input) {
  return <div>{input.title}</div>;
}
```

`Page(input)` is for rendering only. It does not set headers, status, or cookies.

---

## Page Config

### Static config

```tsx
export const page = {
  title: "My Page",
  html: { lang: "en" },
  body: { className: "min-h-screen bg-stone-950 text-stone-100" },
};
```

### Dynamic config

```tsx
export function getPage(input) {
  return {
    title: input.post?.title ?? "Untitled",
  };
}
```

Both are for document metadata only — they don't render body content.

---

## Input Object

`input` is the immutable page input from the pipeline:

```ts
{
  ...upstreamPayload,  // from last pipeline node
  params?,             // URL path params from trigger
  query?,              // query string from trigger
  headers?,            // safe subset of request headers
  auth?                // public JWT claims only
}
```

For static pages with no interactive state, read `input` directly:

```tsx
export default function Page(input) {
  return <h1>{input.rows?.[0]?.title}</h1>;
}
```

---

## Imports

### From `"zeb"` — hooks and utilities

```tsx
import { useState, useEffect, useRef, useMemo, usePageState, useNavigate, Link, cx, tv } from "zeb";
```

This import is required in EVERY file that uses these — entry pages AND components. The compiler strips it at build time (runtime-provided).

### From `"zeb/*"` — subpath modules

```tsx
import { renderMarkdown } from "zeb/markdown";
```

### From `"@/"` — local project files

```tsx
import Button from "@/components/ui/button";
import { formatDate } from "@/scripts/date";
```

`@/` resolves to the template root. Use this for all cross-component imports.

### Import rules

- ALWAYS import from `"zeb"`, never `"preact"`, `"react"`, or `"npm:preact/hooks"`
- ALWAYS use `@/` paths between components, never relative `../`
- NEVER `import { render } from "preact"` — never call `render()` manually
- Directory imports work only if `index.tsx` exists — prefer explicit `@/components/foo/index`

---

## Hooks Reference

### `useState(initial)` — local component state

```tsx
const [open, setOpen] = useState(false);
```

For UI-only state: toggles, selected tabs, form field values.

### `useEffect(fn, deps)` — client-only side effects

```tsx
useEffect(() => {
  const id = setInterval(() => setState(s => s + 1), 1000);
  return () => clearInterval(id);
}, []);
```

Never runs during SSR. Use for: event listeners, data fetching, imperative libraries.

### `useRef(initial)` — DOM reference

```tsx
const containerRef = useRef(null);
useEffect(() => {
  // mount imperative library into containerRef.current
}, []);
return <div ref={containerRef} />;
```

### `useMemo(fn, deps)` — memoized value

```tsx
const sorted = useMemo(() => items.sort((a, b) => a.name.localeCompare(b.name)), [items]);
```

### `usePageState(initial)` — reactive page state

The core SSR→hydration hook:
- SSR: renders with initial data from pipeline
- Client: enables live reactivity via direct mutation

**Object form (most common):**

```tsx
const state = usePageState(input.state ?? { count: 0, items: [] });
state.count++;  // triggers DOM update
```

**Keyed form:**

```tsx
const [count, setCount] = usePageState("count", 0);
setCount(count + 1);
```

**When to use which:**
- `usePageState` → page-specific components with server data
- `useState` → generic reusable UI components (Button, Dialog, etc.)

### `useNavigate()` — programmatic navigation

```tsx
const navigate = useNavigate();
navigate("/posts");  // client-only, no-op during SSR
```

### `Link` — SPA navigation

```tsx
<Link href="/posts/1" className="underline">Read post</Link>
```

Renders as `<a>` during SSR (SEO-friendly), activates client-side routing on hydration.

### `cx(...classes)` — conditional class names

```tsx
<div className={cx("rounded p-4", isActive && "ring-2 ring-accent")} />
```

### `tv(config)` — tailwind variant builder

```tsx
const badge = tv({
  base: "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
  variants: {
    color: {
      success: "bg-green-900/40 text-green-300",
      warning: "bg-amber-900/40 text-amber-300",
    },
  },
  defaultVariants: { color: "success" },
});

// Register variants with compiler (required!)
<span hidden tw-variants="bg-green-900/40 text-green-300 bg-amber-900/40 text-amber-300" />

<span className={badge({ color: "success" })}>Active</span>
```

---

## Styling

### Preferred order

1. Tailwind utility classes in JSX (primary)
2. `styles/main.css` (project-level CSS variables, fonts, semantic tokens)
3. `tw-variants` attribute (dynamic class hints for compiler)
4. `style={{ }}` (dynamic geometry only — width, height, transform)
5. `<style>` blocks (one-off keyframes, sparingly)

### Semantic color tokens

Define in `styles/main.css`:

```css
:root {
  --color-brand-blue: #005b9a;
  --color-surface: #111827;
}
```

Use in TSX:

```tsx
<h1 className="text-brand-blue">Hello</h1>
<div className="bg-surface">Content</div>
```

### Dynamic classes with tw-variants

When classes are computed at runtime, hint the compiler:

```tsx
<span hidden tw-variants="bg-green-900/40 text-green-300 bg-red-900/40 text-red-300" />
```

---

## Components

Components are imported TSX modules:

```tsx
// @/components/user-card.tsx
export function UserCard({ user }) {
  return <div className="rounded-lg border p-4">{user.name}</div>;
}
```

Use design system components when available:

```tsx
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import { Dialog } from "@/components/ui/dialog";
```

Never write raw `<button>`, `<input>` with manual class strings when a `components/ui/` component exists.

---

## Render Flow

```
TSX Module → OXC Compiler → Bundle
  → V8 Worker (SSR) → HTML + hydration payload
    → Browser → Preact hydration → interactive
```

1. Template saved → OXC parses and bundles TSX
2. Pipeline `web.response` node triggers render
3. V8 worker evaluates module: `page` (static config), `getPage(input)` (dynamic config), `Page(input)` (body)
4. HTML document assembled with CSS, hydration script
5. Browser receives HTML, hydrates with Preact

---

## Pipeline → Template Data Flow

```
Pipeline:  trigger.webhook → pg.query → web.response --template pages/posts.tsx
                                ↓
Template:  input = { rows: [...], total: 42 }
           const posts = input?.rows ?? [];
```

The last node's output before `web.response` becomes the template's `input`.
