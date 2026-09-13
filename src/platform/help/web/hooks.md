# `zeb/react` — hooks and helpers

Everything a page or component imports comes from one specifier:

```tsx
import { useState, useEffect, useMemo, cx, Link } from "zeb/react";
```

Import each name in every file that uses it. The compiler lowers a `zeb/react`
import to the runtime's engine object (`globalThis.__zebReact`) or to a
platform global, which is why aliases (`useState as useS`), a default import
(`import React from "zeb/react"`) and a namespace import work too. A hook used
without an import is refused at compile time (`RWE_HOOK_NOT_IMPORTED`).

## Exports

| Name | Kind | Notes |
|---|---|---|
| `useState`, `useReducer`, `useRef`, `useMemo`, `useCallback`, `useEffect`, `useLayoutEffect`, `useContext`, `useId`, `useImperativeHandle`, `useSyncExternalStore` | React hooks | React semantics. `useSyncExternalStore` needs `getServerSnapshot` because pages render on the server first. |
| `createContext`, `createPortal`, `forwardRef`, `memo`, `Fragment` | React API | `createPortal(node, document.body)` for overlays — guard with `typeof document !== "undefined"`. |
| `h`, `createElement`, `jsx`, `jsxs`, `jsxDEV` | element factories | JSX compiles to these; `h(tag, props, ...children)` when you build elements from data. |
| `ErrorBoundary` | component | `fallback` or `fallbackRender({ error, resetErrorBoundary })`, `resetKeys`, `onError`. |
| `render`, `hydrate`, `renderToString` | engine | Never call them from a page — the platform hydrates. |
| `usePageState` | Zebflow | shared page state, below |
| `useRouter`, `usePathname`, `useSearchParams`, `Link` | Zebflow | navigation, below |
| `cx` | Zebflow | `cx("a", cond && "b", other)` → class string; drops falsy parts |

Nothing else exists. In particular there is no `tv()`, no `clsx`, no
`useFetch` — write a plain function or use `useEffect` + `fetch`.

## `usePageState`

State shared by every component on the page, without threading props.

```tsx
// Keyed — the form to use. [value, setter], like useState, but page-wide.
const [tab, setTab] = usePageState("tab", "overview");
const [cart, setCart] = usePageState("cart", []);
```

Any component on the page calling `usePageState("tab", …)` sees the same value
and re-renders when it changes. Server render starts each key at its default;
the browser starts empty and applies defaults the same way, so server and
client agree.

Server data does **not** flow through `usePageState` — it is already in
`input` (see `help("web")`). Read `input.rows` directly; put only client-side
state into page state.

The object form `usePageState()` (no key) returns `{ ...state, setPageState(patch) }`.
It is a plain object, not a proxy: assigning to a property changes nothing.
Prefer the keyed form.

## Navigation

```tsx
import { Link, useRouter, usePathname, useSearchParams } from "zeb/react";

<Link href="/posts/hello" className="underline">Read</Link>

const router = useRouter();
router.push("/posts");        // client-side navigation, history entry
router.replace("/login");     // no history entry
router.back(); router.forward();
router.refresh();             // re-render the current URL without a new entry
router.prefetch("/posts");    // warm the HTTP cache, ignore failures

const pathname = usePathname();          // "/posts/hello" — same on server and client
const params = useSearchParams();        // URLSearchParams; params.get("page")
```

`Link` renders an `<a>` and intercepts the click; the next page is fetched
and swapped in, scroll and history are handled, and a thin progress bar shows
while it loads. `useRouter` has no `pathname` or `query` — that is what
`usePathname` and `useSearchParams` are for.

## `useEffect` and the browser

Effects run only in the browser, so they are the place for `window`,
`document`, timers and `fetch`:

```tsx
import { useEffect, useState } from "zeb/react";

const [items, setItems] = useState(input.items ?? []);
useEffect(() => {
  const id = setInterval(async () => {
    const res = await fetch("/wh/o/p/api/items");
    if (res.ok) setItems(await res.json());
  }, 5000);
  return () => clearInterval(id);
}, []);
```

Do not read `document` or `window` during render — the server has neither.
Dynamic `import()` is refused under the default security policy; load a
runtime library with a static import instead (`import { d3 } from "zeb/d3"`,
see `help("web/libraries")`).

## `cx` and variant maps

```tsx
import { cx } from "zeb/react";

const variants = {
  default: "bg-primary text-primary-foreground hover:bg-primary/90",
  outline: "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
};
<button className={cx("inline-flex h-9 items-center rounded-md px-4 text-sm", variants[variant], className)} />
```

Every class string is a literal, so the Tailwind scan sees it. A class built
by interpolation (`` `bg-${color}` ``) produces no CSS; see `help("web/tailwind")`.
