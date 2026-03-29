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

### `.ts` behavior files — use camelCase exports, never ALL_CAPS

The bundler automatically renames `UPPER_SNAKE_CASE` top-level `const/let/var` declarations with a unique per-file prefix to avoid collisions in the flat output bundle. This means an exported `ALL_CAPS` name is no longer exported under its original name — imports of it resolve to `undefined`.

**Always export camelCase from `.ts` files:**

```ts
// ✓ CORRECT — camelCase, name preserved through bundling
export const apiUrl = "https://api.example.com";
export const defaultPageSize = 20;
export const myConfig = { timeout: 5000 };

// ✗ WRONG — UPPER_SNAKE_CASE gets prefixed, import resolves to undefined
export const API_URL = "https://api.example.com";
export const DEFAULT_PAGE_SIZE = 20;
export const MY_CONFIG = { timeout: 5000 };
```

Consuming page:
```tsx
import { apiUrl, defaultPageSize } from "@/behavior/config";
// ✓ works — camelCase names survive bundling
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
  head: {
    title: "My Page",
    description: "Page description for SEO",
    themeColor: "#145FA4",
    canonical: "https://example.com/my-page",
    robots: "index, follow",
    icons: [
      { rel: "icon",             type: "image/png", sizes: "32x32", href: "/favicon-32x32.png" },
      { rel: "icon",             type: "image/png", sizes: "16x16", href: "/favicon-16x16.png" },
      { rel: "apple-touch-icon", sizes: "180x180",                  href: "/apple-touch-icon.png" },
    ],
    manifest: "/site.webmanifest",
    og: {
      title: "My Page",
      description: "Page description",
      image: "https://example.com/og.png",
      url: "https://example.com/my-page",
      type: "website",
      siteName: "My Site",
    },
    twitter: {
      card: "summary_large_image",
      title: "My Page",
      description: "Page description",
      image: "https://example.com/og.png",
    },
    // extra: raw HTML injected verbatim into <head> — for anything not covered above
    extra: "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">",
  },
  html: { lang: "en" },
  body: { className: "min-h-screen bg-slate-950 text-slate-100" },
};

export const app = {
  hydration: "reactive", // "reactive" | "static" | "none"
};
```

**`page.head` fields:**

| Field | Output | Notes |
|---|---|---|
| `title` | `<title>` | |
| `description` | `<meta name="description">` | |
| `themeColor` | `<meta name="theme-color">` | PWA + browser chrome color |
| `canonical` | `<link rel="canonical">` | SEO deduplication |
| `robots` | `<meta name="robots">` | e.g. `"index, follow"` or `"noindex"` |
| `icons` | `<link rel="icon">` / `<link rel="apple-touch-icon">` | Array of `{ rel, href, type?, sizes? }` |
| `manifest` | `<link rel="manifest">` | PWA web app manifest |
| `og` | `<meta property="og:*">` | `title`, `description`, `image`, `url`, `type`, `siteName`, `locale` |
| `twitter` | `<meta name="twitter:*">` | `card`, `title`, `description`, `image`, `site`, `creator` |
| `extra` | raw HTML verbatim | Escape hatch — inject anything else |

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

The upstream payload passed to **`n.web.response --template`** becomes `input` in the page:

```
pipeline:  trigger → pg.query → web.response --template pages/foo.tsx
                         ↓
template:  input = { rows: [...], row_count: 20 }  (pg.query output)
```

`input` is the function parameter (props). `ctx` is the same object exposed as `globalThis.ctx` — usable in both SSR and browser contexts. Use `input` for server-seeded data, `useState` for client-side state.

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
