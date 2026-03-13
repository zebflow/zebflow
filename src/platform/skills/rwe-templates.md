# RWE Template Authoring

RWE (Reactive Web Engine) compiles TSX templates server-side (SSR) and hydrates them client-side. No build step, no deploy — changes are live immediately after `write_template`.

---

## CRITICAL: Import Rules

### Hooks — globals, no import needed

`useState`, `useEffect`, `useRef`, `useMemo`, `usePageState`, `cx` are **injected as globalThis globals** by the RWE runtime. Just use them.

For the **entry page** only, you may write `import { useState, useEffect, ... } from "rwe"` — the compiler strips it at build time. This is purely for editor type hints.

```tsx
// ✓ CORRECT — entry page (pages/*.tsx)
import { useState, useEffect, useRef, cx } from "rwe";

// ✓ ALSO CORRECT — hooks are globals, no import needed anywhere
const [open, setOpen] = useState(false);
```

```tsx
// ✗ WRONG — NEVER do this
import { useState } from "npm:preact/hooks";
import { useEffect } from "npm:preact";
import { render } from "npm:preact";  // NEVER call render() manually
```

### Component imports — always use `@/` alias

`@/` resolves to the template root at compile time. Always use it. Never use relative paths.

```tsx
// ✓ CORRECT
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import ProjectStudioShell from "@/components/layout/project-studio-shell";

// ✗ WRONG — relative paths break
import Button from "../../components/ui/button";
import Button from "../ui/button";
```

### Component files (non-entry) — no imports at all

Files under `components/` are loaded by Deno directly without stripping. Hooks are already globals. Do not import from `"rwe"`.

```tsx
// components/ui/my-widget.tsx — NO import, hooks just work
export default function MyWidget({ label }) {
  const [open, setOpen] = useState(false);  // ← global, works
  return <button onClick={() => setOpen(!open)}>{label}</button>;
}
```

---

## Page Structure

Every page template exports three things:

```tsx
// pages/my-page.tsx

export default function MyPage(input: PageInput) {
  const state = usePageState(input.state ?? { count: 0, title: "Hello" });

  return (
    <div className="p-8 bg-slate-950 text-slate-100 min-h-screen">
      <h1 className="text-3xl font-bold mb-4">{state.title}</h1>
      <p className="text-slate-400">{state.count}</p>
      <Button onClick={() => state.count++}>Increment</Button>
    </div>
  );
}

export const page = {
  title: "My Page",
  description: "Page description for SEO",
};

export const app = {
  hydration: "reactive",  // "reactive" | "static" | "none"
};
```

**Note:** Always use `className`, not `class`. This is TSX/Preact, not HTML.

---

## `usePageState(initialState)`

Returns a reactive Proxy. On server: renders with the initial snapshot. On client: enables live reactivity — mutations on the state object propagate to the DOM without re-rendering.

```tsx
const state = usePageState(input.state ?? { count: 0, items: [] });

// Mutation triggers reactivity on client
state.count++;
state.items = [...state.items, newItem];
```

Pipeline data flows in via `input.state`. Design your pipeline's final node output to match the shape your template expects.

---

## PageInput Type

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

| Mode | Behaviour | Use when |
|------|-----------|----------|
| `"reactive"` | Full SSR + client JS hydration | Interactive pages (forms, counters, dashboards) |
| `"static"` | SSR only, no client JS | Read-only content, blog posts, landing pages |
| `"none"` | Raw HTML string, no wrapper | Embedded fragments, email templates |

---

## Design System — Always Use `components/ui/`

**Never write raw `<button>`, `<input>`, `<label>` with manual class names.** Always use the ui/ components.

```tsx
import Button from "@/components/ui/button";
import Input from "@/components/ui/input";
import Field from "@/components/ui/field";
import Label from "@/components/ui/label";
import { Select, SelectTrigger, SelectContent, SelectItem } from "@/components/ui/select";
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/card";
import { Dialog, DialogTrigger, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import Badge from "@/components/ui/badge";
import Separator from "@/components/ui/separator";
import Checkbox from "@/components/ui/checkbox";
import Toggle from "@/components/ui/toggle";
import Alert from "@/components/ui/alert";
import { DropdownMenu, DropdownMenuTrigger, DropdownMenuContent, DropdownMenuItem } from "@/components/ui/dropdown-menu";
```

Full list: `button`, `input`, `field`, `label`, `select`, `card` (+ card-header/title/content/footer/description), `dialog` (+ sub-parts), `tabs` (+ sub-parts), `badge`, `separator`, `checkbox`, `toggle`, `alert`, `dropdown-menu` (+ sub-parts), `kbd`, `markdown`, `code-editor`

---

## Tailwind + tw-variants

Use Tailwind utility classes. For dynamic class combinations use `cx()` (always available as a global):

```tsx
<div className={cx("rounded-lg p-4", isActive && "bg-sky-900", disabled && "opacity-50")}>
```

For components with many variant permutations, define a `tv()` variant map:

```tsx
const badge = tv({
  base: "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
  variants: {
    variant: {
      default: "bg-slate-800 text-slate-200",
      success: "bg-green-900 text-green-200",
      warning: "bg-amber-900 text-amber-200",
      danger:  "bg-red-900 text-red-200",
    },
  },
  defaultVariants: { variant: "default" },
});
```

---

## Layout Files

Layout files under `components/layout/` wrap every page. The platform layout is `project-studio-shell.tsx`. To add a component that appears on every admin page, put it in the layout — not in individual pages.

```tsx
import ProjectStudioShell from "@/components/layout/project-studio-shell";

export default function MyPage(input) {
  const state = usePageState(input.state ?? {});
  return (
    <ProjectStudioShell input={input}>
      <div className="p-6">...</div>
    </ProjectStudioShell>
  );
}
```

---

## Third-Party Libraries (CodeMirror, D3, Chart.js, etc.)

Use `useEffect` + dynamic `import()`. Never import at module top-level — it breaks SSR.

```tsx
export default function EditorPage(input) {
  const editorRef = useRef(null);

  useEffect(() => {
    // Runs on client only after mount
    import('/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs')
      .then(({ EditorView, basicSetup }) => {
        new EditorView({ extensions: [basicSetup], parent: editorRef.current });
      });
  }, []);

  return <div ref={editorRef} className="h-full" />;
}
```

---

## MCP Workflow

```
1. create_template  kind=page  name=my-page
   → scaffolds pages/my-page.tsx with boilerplate

2. get_template  rel_path=pages/my-page.tsx
   → read the scaffold to understand structure

3. write_template  rel_path=pages/my-page.tsx  content="..."
   → overwrite with your TSX

4. register_pipeline + activate_pipeline
   → serve the page at a live route via web.render
```

Template kinds: `page`, `component`, `script`, `folder`

---

## Pipeline → Template Data Flow

The last node before `web.render` determines what `input.state` contains in your template:

```
pipeline:  trigger → pg.query → web.render
                         ↓
template:  input.state = { rows: [...] }  (pg.query output)
```

Design your template's `usePageState` initializer to match the pipeline output shape.
