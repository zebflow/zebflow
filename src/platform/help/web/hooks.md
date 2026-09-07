# Zeb Hooks — Import Contract

Core UI hooks can be imported from `zeb/react`. Existing imports from `zeb/react`
remain supported. Zebflow helpers (`usePageState`, `useRouter`, `Link`, `cx`)
continue to come from `zeb/react`.

All Zeb hooks and utilities are imported explicitly from `"zeb/react"` (or a `"zeb/*"` subpath).
**There are no implicit globals** — always write the import, in every file that uses them.

The compiler strips these imports at build time (they are runtime-provided), so they never
appear in the final bundle as a real module specifier. They exist purely as an explicit contract
in source code — for readability, IDE type hints, and linting.

```tsx
import { useState, useEffect, useRef, useMemo } from "zeb/react";
import { usePageState, useRouter, Link, cx, tv } from "zeb/react";
```

This applies to **all template files** — entry pages (`pages/*.tsx`) and component files
(`components/**/*.tsx`) alike. The compiler strips the import in both cases.

---

## Available Exports

| Name | Kind | Description |
|------|------|-------------|
| `useState` | hook | Local component state |
| `useEffect` | hook | Side effects (client-only; runs after mount) |
| `useRef` | hook | DOM element reference / stable mutable value |
| `useMemo` | hook | Memoised computed value |
| `useSyncExternalStore` | hook | Subscribe to external state with an SSR snapshot (`zeb/react`) |
| `ErrorBoundary` | component | Isolate failures and retry a subtree (`zeb/react`) |
| `usePageState` | hook | Reactive page-level state (SSR + hydration) |
| `useRouter` | hook | SPA navigation function (client only) |
| `Link` | component | `<a>` wrapper for SPA navigation |
| `cx` | function | Class name concatenation |
| `tv` | function | `tailwind-variants` variant map builder |

---

## Error recovery and external stores

Import these APIs from `zeb/react`:

```tsx
import { ErrorBoundary, useSyncExternalStore } from "zeb/react";
```

Wrap a panel in `<ErrorBoundary fallback={<p>Panel unavailable</p>}>` or use
`fallbackRender={({ error, resetErrorBoundary }) => ...}` for a retry button.
`onError(error, { componentStack })` reports each failure; optional `onReset`
repairs its cause before retrying. Changing `resetKeys={[documentId]}` retries
when a different document is selected. Children remount with fresh state.

This is Zeb's function-component runtime boundary, not React's class API. It
catches descendant rendering, effects, refs and cleanup, including SSR fallback
rendering. Fallback failures escalate to a parent boundary. Event handlers,
promise rejections and independent timer callbacks need their own error handling.

`useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)` reads an
external store and updates its subscribers synchronously when the store notifies
them. `subscribe` must return an unsubscribe function. Keep object snapshots
immutable and cached until the value changes; comparisons use `Object.is`.
`getServerSnapshot` is required for SSR/static generation and hydration, with
the same initial value on server and browser. SSR never subscribes. Hydration
starts with that server value and then catches up to the current browser store.

## `useState(initial)` — local component state

Zeb React `useState`. Use for UI-only state (open/closed toggle, selected tab, etc.):

```tsx
import { useState } from "zeb/react";

const [open, setOpen] = useState(false);
const [text, setText] = useState("");

return <button type="button" onClick={() => setOpen(!open)}>{open ? "Close" : "Open"}</button>;
```

---

## `useEffect(fn, deps)` — side effects / client-only code

Runs **on the client after mount**. Never runs during SSR. Use for:
- Mounting imperative libraries (d3, codemirror, three.js)
- Setting up event listeners / subscriptions
- Fetching data after initial render

```tsx
import { useEffect } from "zeb/react";

useEffect(() => {
  const id = setInterval(() => setState(s => s + 1), 1000);
  return () => clearInterval(id);  // cleanup
}, []);  // [] = run once on mount
```

---

## `useRef(initial)` — DOM reference / stable mutable value

```tsx
import { useRef, useEffect } from "zeb/react";

const containerRef = useRef<HTMLDivElement>(null);

useEffect(() => {
  import('/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs')
    .then(({ EditorView, presets }) => {
      new EditorView({ extensions: presets.zebflow({ kind: "text" }), parent: containerRef.current });
    });
}, []);

return <div ref={containerRef} className="h-64" />;
```

---

## `usePageState(initial)` — reactive page state

`usePageState` is Zeb's core SSR→hydration hook. It:
- On **server (SSR)**: renders with initial snapshot from pipeline `input`
- On **client (hydration)**: enables live reactivity — mutations propagate to DOM without re-rendering the whole tree

### When to use `usePageState` vs `useState`

**`usePageState`** — for **page-specific components**: parts of a page that are unique to that
page and need to reflect server-fetched data or share state across the page tree.
Examples: a post list, a dashboard widget, a form that submits and shows results.

**`useState`** — for **generic reusable components**: UI primitives that only manage their own
local interaction state and have no concept of page-level data.
Examples: `<Button>`, `<Card>`, `<Dialog>`, `<Dropdown>`, `<Tooltip>`, `<Badge>`.

> Rule: anything in a shared component catalog (`components/ui/`) should use only `useState`.
> `usePageState` belongs in page components and page-specific sub-components.

### Object form (most common)

```tsx
import { usePageState } from "zeb/react";

const state = usePageState(input.state ?? { count: 0, items: [], title: "Page" });

// Direct mutation triggers DOM update on client
state.count++;
state.title = "Updated";
state.items = [...state.items, newItem];
```

### Keyed form (isolate one field)

```tsx
import { usePageState } from "zeb/react";

const [count, setCount] = usePageState("count", 0);
const [title, setTitle] = usePageState("title", "Hello");

setCount(count + 1);
setTitle("Updated");
```

### Pipeline → template data flow

Design `usePageState` to match what your pipeline's last node outputs:

```
pipeline:  trigger.webhook → pg.query → web.response --template pages/posts.tsx
                               ↓
template:  input = { rows: [...], total: 42 }
           const state = usePageState(input.state ?? { rows: [], total: 0 });
           // OR access directly: const posts = input?.rows ?? [];
```

For **static pages** (no interactive state needed), skip `usePageState` and read `input` directly.

---

## `useRouter()` — programmatic SPA navigation

Returns a function. Works on client only (no-op during SSR).

```tsx
import { useRouter } from "zeb/react";

const navigate = useRouter().push;

async function handleSubmit(e) {
  e.preventDefault();
  await fetch("/api/posts", { method: "POST", body: JSON.stringify(data) });
  navigate("/posts");  // redirect after submit
}
```

---

## `Link` — `<a>` with SPA routing

Renders as a plain `<a>` during SSR (SEO-friendly), activates client-side routing on hydration.

```tsx
import { Link, cx } from "zeb/react";

<Link href="/posts/1" className="underline hover:text-accent">Read post</Link>
<Link href="/admin" className={cx("px-4 py-2 rounded", isActive && "bg-surface-2")}>Admin</Link>
```

---

## `cx(...classes)` — conditional class names

```tsx
import { cx } from "zeb/react";

<div className={cx("rounded p-4", isActive && "ring-2 ring-accent")}>

<button className={cx(
  "px-4 py-2 rounded font-medium transition",
  variant === "primary" && "bg-accent text-white hover:bg-accent-strong",
  variant === "ghost"   && "bg-transparent text-body hover:bg-surface-3",
  disabled              && "opacity-50 cursor-not-allowed pointer-events-none",
)}>
```

---

## `tv(config)` — variant map builder

```tsx
import { tv } from "zeb/react";

const badge = tv({
  base: "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
  variants: {
    color: {
      default: "bg-surface-3 text-body",
      success: "bg-green-900/40 text-green-300",
      warning: "bg-amber-900/40 text-amber-300",
      danger:  "bg-red-900/40 text-red-300",
    },
  },
  defaultVariants: { color: "default" },
});

// Register all variant strings with the engine (required!)
<span hidden tw-variants="bg-surface-3 text-body bg-green-900/40 text-green-300 bg-amber-900/40 text-amber-300 bg-red-900/40 text-red-300" />

// Usage
<span className={badge({ color: "success" })}>Active</span>
```

See **`help_docs topic=tailwind`** for the full `tw-variants` explanation.

## `usePathname()` — the current path

```tsx
import { usePathname, Link, cx } from "zeb/react";

const pathname = usePathname();
<Link href="/settings" className={cx("tab", pathname === "/settings" && "tab-active")}>
  Settings
</Link>
```

Returns a string. It subscribes, so it stays correct after a `Link` click, which
changes the URL without reloading the page — reading `window.location.pathname`
directly would keep showing the previous path.

During server rendering it returns the route the request arrived on, so the
server and the first browser render agree.

## `useSearchParams()` — the query string

```tsx
import { useSearchParams } from "zeb/react";

// URL: /posts?tag=travel&page=2
const params = useSearchParams();
params.get("tag");   // "travel"
params.get("page");  // "2"
```

Returns a read-only search-parameter view with `get`, `getAll`, `has`, `entries`,
`keys`, `values`, `forEach`, `size`, iteration and `toString`, and subscribes the
same way. Server rendering preserves repeated keys and decodes percent escapes
and `+` from the original request query. Direct renderers can supply `search`
or a decoded `query` object; only `search` preserves repeated keys. To *write*
search params, navigate:

```tsx
const router = useRouter();
router.replace(`${pathname}?tag=food`);
```
