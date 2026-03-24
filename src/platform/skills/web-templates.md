# Web templates (TSX pages)

Zebflow serves HTML from **TSX files** in your project: the server renders them to HTML, then the browser can hydrate for interactivity. After you save with `template_write`, the next request uses the new file — no separate frontend build in the repo.

---

## Imports you are allowed to use

1. **`@/…`** — your project files under `repo/pipelines/` (pages, components, `shared/ui`, …). Always use this alias; do not use `../../` paths.

2. **`"zeb"`** — optional line at the top of a **page** file only, for editor hints, e.g. `import { useState } from "zeb"`. The compiler removes these lines; hooks still exist as globals at runtime.

3. **`"zeb/…"`** — optional add-on libraries enabled in the project (**Settings → Libraries**). Each maps to one bundled script.

Anything else (npm URLs, relative imports, random URLs) is not allowed for normal pages.

```tsx
// ✓ Page file — optional hint import (stripped)
import { useState, cx } from "zeb";

// ✓ Hooks work with or without that import
const [open, setOpen] = useState(false);
```

```tsx
// ✗ Do not use
import { useState } from "npm:preact/hooks";
import { render } from "npm:preact";
```

### Shared components

Hooks are globals. You do not need a top import in every file.

```tsx
export default function MyWidget({ label }) {
  const [open, setOpen] = useState(false);
  return <button type="button" onClick={() => setOpen(!open)}>{label}</button>;
}
```

---

## Page file shape

Default export is the page. You usually also export `page` (SEO / document) and `app` (hydration mode):

```tsx
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
  html: { lang: "en" },
};

export const app = {
  hydration: "reactive", // "reactive" | "static" | "none"
};
```

Use **`className`**, not `class`.

---

## `usePageState`

Often fed from **`input.state`**, which should match what your pipeline passes into `n.web.render`.

---

## `PageInput` (typical)

```ts
interface PageInput {
  state?: Record<string, unknown>;
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

## Hydration modes

| `app.hydration` | When |
|-----------------|------|
| `"reactive"` | Forms, dashboards, anything that updates in the browser |
| `"static"` | Mostly read-only HTML |
| `"none"` | Rare; fragments / special cases |

---

## Buttons, inputs, cards — `shared/ui/`

Install the UI kit into **`repo/pipelines/shared/ui/`** (studio installer or MCP `install_ui_components`). Import like:

`import Button from "@/shared/ui/button";`

The **platform studio** UI lives in a different tree; do not assume `@/components/ui/...` exists in your project unless you created those paths.

---

## Tailwind and `cx()`

`cx()` is a global for joining class names.

---

## Heavy or browser-only code

If you need a big library, prefer an enabled **`zeb/...`** bundle. If you must load code only in the browser, use `useEffect` + dynamic `import()` so SSR does not run it at module load time.

---

## MCP flow

```
template_create   kind=page   name=my-page
template_get      rel_path=pages/my-page.tsx
template_write    rel_path=pages/my-page.tsx   content="..."
pipeline_register / pipeline_activate
```

Before writing TSX, call **`help_web_engine`** or **`skill_read web-templates`**.

---

## Data from the pipeline

The object reaching **`n.web.render`** becomes the page’s **`input`**. Shape `usePageState(input.state ?? …)` to match what the previous nodes output.
