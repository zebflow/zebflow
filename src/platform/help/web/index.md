# Web Templates (TSX Pages)

Zebflow serves HTML from **TSX files** in your project: the server renders them to HTML (SSR), then the browser hydrates for interactivity. After you save with `template_write`, the next request uses the new file — no separate frontend build.

---

## Import Rules (CRITICAL)

### Hooks — globals, no import needed

`useState`, `useEffect`, `useRef`, `useMemo`, `usePageState`, `cx` are **injected as globalThis globals** by the RWE runtime. Just use them.

For the **entry page** only, you may write `import { useState } from "zeb"` — the compiler strips it. This is for editor type hints only.

```tsx
// ✓ CORRECT — entry page (pages/*.tsx), import stripped at compile time
import { useState, useEffect, cx } from "zeb";

// ✓ ALSO CORRECT — hooks are globals, no import needed
const [open, setOpen] = useState(false);
```

```tsx
// ✗ WRONG — NEVER do this
import { useState } from "npm:preact/hooks";
import { render } from "npm:preact";  // NEVER call render() manually
```

### Component imports — always use `@/` alias

`@/` resolves to the template root at compile time. Always use it. Never use relative paths.

```tsx
// ✓ CORRECT
import Button from "@/shared/ui/button";
import MyWidget from "@/components/my-widget";

// ✗ WRONG — relative paths break
import Button from "../../components/ui/button";
```

### Component files (non-entry) — no imports for hooks

Files under `components/` are loaded without stripping. Hooks are already globals. Do not import from `"zeb"` or `"rwe"` in component files.

```tsx
// components/my-widget.tsx — NO import, hooks just work
export default function MyWidget({ label }) {
  const [open, setOpen] = useState(false);  // ← global, works
  return <button onClick={() => setOpen(!open)}>{label}</button>;
}
```

---

## Page File Shape

```tsx
// pages/my-page.tsx

export default function MyPage(input: PageInput) {
  const state = usePageState(input.state ?? { count: 0, title: "Hello" });

  return (
    <div className="p-8 bg-slate-950 text-slate-100 min-h-screen">
      <h1 className="text-3xl font-bold mb-4">{state.title}</h1>
      <p className="text-slate-400">{state.count}</p>
    </div>
  );
}

export const page = {
  head: { title: "My Page" },
};

export const app = {
  hydration: "reactive", // "reactive" | "static" | "none"
};
```

Use **`className`**, not `class`.

---

## `usePageState(initialState)`

Returns a reactive Proxy. On server: renders with the initial snapshot. On client: live reactivity — mutations propagate to the DOM.

```tsx
const state = usePageState(input.state ?? { count: 0, items: [] });
state.count++;           // ← triggers DOM update on client
state.items = [...state.items, newItem];
```

Pipeline data flows in via `input.state`. Design your pipeline's final node output to match the shape your template expects.

---

## `PageInput` Type

```ts
interface PageInput {
  state?: Record<string, unknown>;  // data from pipeline's last node
  request?: {
    method: string;
    path: string;
    query: Record<string, string>;
    headers: Record<string, string>;
    body?: unknown;
  };
}
```

---

## Hydration Modes

| `app.hydration` | Behaviour | Use when |
|-----------------|-----------|----------|
| `"reactive"` | Full SSR + client JS hydration | Interactive pages (forms, dashboards) |
| `"static"` | SSR only, no client JS | Read-only content, blog posts |
| `"none"` | Raw HTML string, no wrapper | Fragments, email templates |

---

## Data from the Pipeline

The object reaching **`n.web.render`** becomes the page's **`input`**:

```
pipeline:  trigger → pg.query → web.render
                         ↓
template:  input.state = { rows: [...] }  (pg.query output)
```

---

## MCP Workflow

```
template_create   kind=page   name=my-page
template_get      rel_path=pages/my-page.tsx
template_write    rel_path=pages/my-page.tsx   content="..."
pipeline_register + pipeline_activate
```

---

## Further Reading

- `help("web/hooks")` — useState, useEffect, usePageState, cx, useNavigate, Link, tv
- `help("web/tailwind")` — semantic tokens, tw-variants, cx(), tv()
- `help("web/libraries")` — zeb/* bundled add-ons: icons, markdown, codemirror, d3
- `help("web/design-system")` — component library rules
