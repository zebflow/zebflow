# RWE — Reactive Web Engine Specification

> **Design principle:** A developer who knows React should be able to open an RWE file and feel at home.
> No new mental model. No custom directives. No magic strings. Just TSX, imports, and standard hooks.

- from {bla, bla, bla} import "rwe" means this bla, bla, bla need to be added into the compiled

---

## Legend

| Symbol | Meaning |
|--------|---------|
| ✅ | Implemented and working |
| ⚠️ | Partially implemented — needs fix/completion |
| 🔨 | To be built now |
| 🔮 | Planned for later milestone |
| ❌ | Omitted / out of scope |

---

## 0. Complete Format Reference

### Backend (Rust)

```rust
// --- COMPILE ONCE AT STARTUP ---
insert_compiled_page(
    "home",                                               // page key
    "page.home",                                          // template id
    include_str!("demo/templates/pages/home.tsx"),        // markup source
    ReactiveWebOptions {
        processors: vec!["tailwind".to_string()],
        components: ComponentOptions {
            registry: {
                let mut m = BTreeMap::new();
                m.insert("MyComp".to_string(), include_str!("components/my-comp.tsx").to_string());
                m
            },
            strict: true,
        },
        allow_list: ResourceAllowList {
            scripts: vec!["https://cdn.example.com/lib.js".to_string()],
            ..Default::default()
        },
        load_scripts: vec!["https://cdn.example.com/lib.js".to_string()],
        ..Default::default()
    },
)?;

// --- RENDER PER REQUEST ---
async fn route_home(State(state): State<DemoAppState>) -> Html<String> {
    render_page(&state, "home", "/", json!({ "user": "mala", "count": 3 }))
        .map(Html)
        .map_err(internal_error)
}
```

### Frontend (`home.tsx`)

```tsx
import { useState, useEffect, useRef, useMemo, usePageState, useNavigate, Link } from "rwe";

export const page = {
  head: { title: "My Page" },
  navigation: "history",
};

export default function Page(input) {
  const [localVal, setLocalVal] = useState(input.count ?? 0);
  const [name, setName] = useState(input.user ?? "");
  const ref = useRef(null);
  const shared = usePageState({ count: 0 });
  const navigate = useNavigate();

  useEffect(() => {
    shared.setPageState({ count: localVal });
  }, [localVal]);

  const doubled = useMemo(() => localVal * 2, [localVal]);

  return (
    <div class="p-4 bg-zinc-900 text-white">
      <h1>Hello, {name}</h1>
      <p>Local: {localVal}</p>
      <p>Doubled: {doubled}</p>
      <p>Shared count: {shared.count}</p>
      <button onClick={() => setLocalVal(localVal + 1)}>+1</button>
      <button onClick={() => navigate("/other")}>go</button>
      <Link href="/about">About</Link>
      <div ref={ref}>tracked element</div>
    </div>
  );
}
```

---

## 1. Technical Architecture

### 1.1 The Two Worlds

RWE operates in two distinct worlds that must stay coherent:

```
┌─────────────────────────────────────────────────────────────┐
│  SERVER (Rust + embedded deno_core)                          │
│                                                              │
│  Request → compile() → render_ssr() → HTML string           │
│                                                              │
│  Embedded V8 via deno_core (singleton JsRuntime thread)     │
│  preact_ssr_init.js loaded ONCE — installs all globals      │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  CLIENT (Browser)                                            │
│                                                              │
│  HTML lands → <script type=module> hydrates #__rwe_root     │
│                                                              │
│  build_client_module() bakes runtime globals inline         │
│  No extra round-trip. No CDN dependency at runtime.         │
└─────────────────────────────────────────────────────────────┘
```

---

### 1.2 Compile Pipeline

```
Source .tsx file (entry page)
        │
        ▼
┌──────────────────┐
│   OXC Parser     │  Parse TSX into AST (Rust, fast)
│  (Rust, oxc)     │  Panics → saved to /tmp/rwe-parse-failed.tsx
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ ensure_default_  │  Must have `export default function Page()`
│    export()      │  Fails hard if missing
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ security::       │  Scan for forbidden patterns
│   analyze()      │  (eval, dangerous DOM access, etc.)
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ collect_imports  │  Gather all import sources from OXC AST
│ validate_        │  Check against allowlist:
│   allowlist()    │    "rwe", "npm:*", "node:*", "jsr:*", "@/*", "/*"
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ rewrite_imports  │  @/ → absolute filesystem path
│ (alias rewrite)  │  Collects ImportEdge list (source + resolved)
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ bundle_for_      │  Recursively inline ALL local component files
│   client()       │  into ONE self-contained module.
│                  │
│                  │  Per-component text pipeline (each inlined file):
│                  │
│                  │  1. strip_local_imports()        ← runs on ORIGINAL content
│                  │       Remove `from "rwe"` and filesystem imports.
│                  │       Uses OXC AST byte spans — handles multi-line imports.
│                  │       Must run first — import paths must be visible
│                  │       (masking them would prevent correct filtering).
│                  │
│                  │  2. mask_string_literals()
│                  │       Replace all string/template literal contents
│                  │       with opaque placeholders __RWE_MASK_0__ etc.
│                  │       Compresses multiline template literals to one line.
│                  │       (e.g. `import x from y` inside a template is safe)
│                  │
│                  │  3. localize_exports()
│                  │       `export default function X` → `function X`
│                  │       `export default class X` → `class X`
│                  │       `export default Select;` → `Select;` (bare re-export)
│                  │       `export type/interface` → stripped (with multi-line
│                  │         brace-depth tracking for multi-line type defs)
│                  │       Safe: multiline template content is now masked.
│                  │
│                  │  4. prefix_module_locals()
│                  │       Auto-prefix UPPER_SNAKE_CASE module-scope constants
│                  │       with __c{n}_ per component — no collision in flat bundle
│                  │       (e.g. VARIANT_CLASSES → __c0_VARIANT_CLASSES)
│                  │       Safe: string contents are masked, so COLORS inside
│                  │       a string literal is never wrongly prefixed.
│                  │
│                  │  5. unmask_string_literals()
│                  │       Restore all __RWE_MASK_N__ → original content
│                  │
│                  │  Result: zero filesystem imports, zero "rwe" imports,
│                  │  no naming collisions, user string content untouched
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ JSX_PRELUDE      │  Prepend `/** @jsxImportSource npm:preact */`
│  inject          │
└────────┬─────────┘
         │
         ▼
   CompiledTemplate
   ├── server_module_source  (fully bundled, for embedded deno_core SSR)
   ├── client_module_source  (fully bundled, for browser hydration)
   └── imports, diagnostics, hydrate_mode

BOTH server and client get the same fully-inlined bundle.
At render time there are NO filesystem imports and NO "rwe" imports.
Everything is compiled. Runtime globals handle the rest.
```

#### String Masking — Why It Matters

All text transforms (strip imports, localize exports, prefix constants) operate on raw
text, not AST. Without masking, a string like:

```ts
const SNIPPET = `import Button from "@/components/ui/button"`;
const SQL = "import xx from yy; select * from users";
```

would have its content incorrectly stripped or mutated by the line-based transforms.

**Masking contract:**
- Masks `"..."`, `'...'`, and `` `...` `` (template literals, including nested `${...}`)
- Placeholders: `"__RWE_MASK_0__"`, `'__RWE_MASK_1__'`, `` `__RWE_MASK_2__` ``
  (quotes preserved so surrounding syntax stays valid)
- Restored verbatim after all transforms complete
- Implementation status: ✅ implemented (`compiler.rs` — `mask_string_literals` / `unmask_string_literals`)

---

### 1.3 Render Pipeline

```
CompiledTemplate + vars (JSON)
        │
        ├──► render_ssr(server_module_source, vars)
        │         │
        │         ▼
        │    Embedded deno_core (singleton JsRuntime on dedicated thread)
        │    ├── preact_ssr_init.js loaded ONCE at startup — installs globals:
        │    │     h, Fragment, React, createElement,
        │    │     useState, useEffect, useLayoutEffect, useInsertionEffect,
        │    │     useRef, useMemo, useCallback, useContext, useReducer, useId,
        │    │     useImperativeHandle, forwardRef, memo, createContext,
        │    │     usePageState, useNavigate, Link, cx
        │    │     __rweRenderToString, __rweWrapWithPageState
        │    ├── transpile_tsx() — OXC strips TS/JSX → plain JS (h() calls)
        │    ├── strip_rwe_imports() — remove any remaining "rwe" import lines
        │    ├── inject_page_globals() — expose default export on globalThis
        │    │     globalThis.__rwe_page = <default export name>
        │    │     globalThis.__rwe_page_config = page config (if defined)
        │    ├── globalThis.ctx = vars (inject render vars before module load)
        │    ├── load_side_es_module() — load fully-bundled module (no ext. files)
        │    │     (side module, not main — allows multiple pages per runtime)
        │    └── execute render script → op_rwe_store_result(html+config JSON)
        │
        ├──► transpile_client_cached(client_module_source)
        │         │
        │         ▼
        │    OXC transpiles TSX → JS via deno_core transpile_client()
        │    Result cached by source hash (in-memory, 256 cap LRU-ish)
        │
        └──► build_client_module(transpiled_js)
                  │
                  ▼
             Inline preamble injected (NO extra HTTP requests):
             ├── import preact + hooks from esm.sh (pinned 10.28.4)
             ├── globalThis.h, Fragment, React, cx
             ├── globalThis.useState, useEffect, useRef, useMemo
             ├── globalThis.usePageState = __rweUsePageState
             ├── globalThis.useNavigate (calls window.rweNavigate)
             ├── globalThis.Link (intercepts click → window.rweNavigate)
             ├── window.rweNavigate — inline SPA router (installed once per page load)
             │     fetch → DOMParser → full DOM patch:
             │       #__rwe_root innerHTML swap, #__rwe_payload swap
             │       <style data-rwe-tw> swap (per-page Tailwind CSS)
             │       <link rel="stylesheet"> sync (per-page extra sheets)
             │       body.className swap, html.lang swap, document.title swap
             │       old nav scripts removed, new module scripts executed
             │     history.pushState, popstate handler, rwe:nav event
             │     Progress bar (#__rwe_nav_bar, --rwe-nav-color CSS variable)
             ├── base64-encode page module → data: URL import
             └── hydrate(<Page>, #__rwe_root)
```

---

### 1.4 Temp Dir & Asset Materialization

```
Binary (embedded via build.rs)
        │
        ▼
materialize_platform_template_root()
        │
        ├── Debug mode: always re-extract + delete stale files ✅
        ├── Release mode: skip if .materialized sentinel exists
        └── rewrite_platform_template_alias_imports() on all files
              @/ → absolute temp dir path in every .tsx/.ts file
```

---

### 1.5 Embedded JS Runtime

```
static JS_CHANNEL: LazyLock<UnboundedSender<JsRequest>>
```

- **Embedded deno_core** — V8 runs in-process, no external `deno` binary needed
- **Single dedicated thread** — `JsRuntime` is `!Send`, lives on `rwe-js-runtime` thread
- **Singleton runtime** — `preact_ssr_init.js` loaded once at startup; globals persist
- **Side modules** — `load_side_es_module()` used (not main) so runtime can render multiple pages
- **Custom op** — `op_rwe_store_result(json)` delivers rendered HTML from JS→Rust via thread-local slot
- **Custom module loader** — `RweModuleLoader` resolves file:// URLs, transpiles TSX/TS on-the-fly via OXC
- Restart server = fresh runtime (important after template changes in dev)
- Client transpile: OXC in-process (Rust, no JS runtime needed)

**Runtime files:**
| File | Purpose | Used by |
|------|---------|---------|
| `runtime/preact_ssr_init.js` | Self-contained SSR globals + renderToString | Embedded deno_core (current) |
| `runtime/ssr_worker.mjs` | External Deno subprocess worker (stdin/stdout JSON) | Legacy — not used by current engine |

---

## 2. Developer Experience Rules

### 2.1 React-Compatible Syntax

| Rule | Detail | Status |
|------|--------|--------|
| JSX in `.tsx` files | Standard JSX, PascalCase components | ✅ |
| Functional components with props | `function Page(props) { return <div/> }` | ✅ |
| `export default` page component | Required — compile fails without it | ✅ |
| Fragments `<>...</>` | Standard fragment syntax | ✅ |
| Conditional rendering `{x && <Y/>}` | Standard JSX patterns | ✅ |
| List rendering `.map((x) => <Item/>)` | Standard JSX patterns | ✅ |
| Event handlers `onClick`, `onInput`, etc. | Standard JSX events | ✅ |
| Preact internals hidden | Developer never imports from `npm:preact` directly | ✅ |

---

### 2.2 The `"rwe"` Module — Hooks & Utilities

Everything a developer needs comes from a single import: `import { ... } from "rwe"`.

This works in **every file** — pages, components, layouts, behaviors. No exceptions.

| Export | Description | Status |
|--------|-------------|--------|
| `useState` | Local component state | ✅ |
| `useEffect` | Side effects after render | ✅ |
| `useLayoutEffect` | Synchronous layout effects (SSR no-op) | ✅ |
| `useRef` | Mutable ref to DOM element | ✅ |
| `useMemo` | Memoized computed value | ✅ |
| `useCallback` | Memoized callback function | ✅ |
| `useContext` | Consume a React-style context | ✅ |
| `useReducer` | Reducer-based state management | ✅ |
| `useId` | Stable unique ID for SSR/client matching | ✅ |
| `useImperativeHandle` | Customize ref handle (SSR no-op) | ✅ |
| `createContext` | Create a React-style context | ✅ |
| `forwardRef` | Forward refs through components | ✅ |
| `memo` | Memoize component (identity passthrough in SSR) | ✅ |
| `usePageState` | Shared state across all components on the same page | ✅ |
| `useNavigate` | SPA navigation hook — `const nav = useNavigate(); nav("/path")` | ✅ |
| `Link` | Router-aware anchor — `<Link href="/path">Go</Link>` | ✅ |
| `cx` | Class name utility — `cx("base", condition && "extra", className)` | ✅ |

**How `"rwe"` imports work — compile-time signal:**

`import { cx, useState } from "rwe"` is a **signal to the compiler**, not a real module import.

- At **compile time**: `bundle_for_client()` calls `strip_local_imports()` which strips all `from "rwe"` lines from every inlined component. They never reach the runtime.
- At **runtime**: all exported symbols are already installed as `globalThis.*` by the runtime — `preact_ssr_init.js` on the server, `build_client_module()` preamble on the client.
- **Type definitions**: `rwe.d.ts` + `tsconfig.json` path mapping — planned, enables IDE autocomplete. Not required for runtime to work.

This pattern works in **every file** — pages, components, layouts. No exceptions.

---

### 2.3 Navigation

| Rule | Detail | Status |
|------|--------|--------|
| SPA navigation via `useNavigate()` | `const nav = useNavigate(); nav("/projects/x")` | ✅ |
| `<Link href="...">` component | Intercepts click → `window.rweNavigate(href)` | ✅ |
| Inline SPA router in every page | `window.rweNavigate` baked into `build_client_module()` — single source of truth, no extra files | ✅ |
| `history.pushState` URL updates | Browser URL bar reflects current page without reload | ✅ |
| Back/forward browser buttons | `popstate` handler re-runs `window.rweNavigate` | ✅ |
| Per-page Tailwind CSS swap | `<style data-rwe-tw>` removed/re-added on every navigation — selector matches server injection | ✅ |
| `<link rel="stylesheet">` sync | Per-page extra sheets (db-suite, devicons) added/removed on navigation | ✅ |
| `body.className` swap | Dark/light layout class updated on every navigation | ✅ |
| `html.lang` swap | Language attribute updated on navigation | ✅ |
| Page title update on navigation | `document.title` set from fetched page | ✅ |
| `rwe:nav` event | Dispatched after swap completes — for analytics, behavior re-init | ✅ |
| Progress bar | Thin top bar, `#__rwe_nav_bar`, color via `--rwe-nav-color` CSS var | ✅ |
| `rwe_router.js` | ❌ Removed — was a legacy shell-specific router, fully replaced by inline system | ❌ deleted |
| Direct `window.location.href` | ❌ Never — raw DOM access forbidden in TSX/TS source | ❌ omit |
| `window.rweNavigate` in user code | ❌ Internal only — use `useNavigate()` or `<Link>` | ❌ omit |

---

### 2.4 File Modularity

| Rule | Detail | Status |
|------|--------|--------|
| `import X from "@/components/ui/button"` | `@/` resolves to template root, works in all files | ✅ |
| `import X from "@/components/layout/shell"` | Nested imports cascade correctly | ✅ |
| No relative `../` imports | `../../foo` is forbidden — use `@/` always | ✅ enforced |
| `import X from "npm:somelib"` | Allowed npm packages per allowlist | ✅ |
| `import X from "jsr:somelib"` | JSR packages allowed | ✅ |
| Component dependency graph / cache | Hash-based cache per source file | 🔮 M1 |
| Cycle detection in imports | Detect and error on circular deps | 🔮 M1 |

---

### 2.5 Import Allowlist (Sandbox)

Developers can only import from these sources. Everything else is blocked at compile time.

| Source | Example | Status |
|--------|---------|--------|
| `"rwe"` | `import { useState } from "rwe"` | ✅ |
| `"npm:*"` | `import { z } from "npm:zod"` | ✅ |
| `"node:*"` | `import { Buffer } from "node:buffer"` | ✅ |
| `"jsr:*"` | `import x from "jsr:@std/fmt"` | ✅ |
| `"@/*"` | `import X from "@/components/ui/x"` | ✅ |
| Custom allowlist prefixes | Configured per project security policy | ✅ |
| Arbitrary HTTPS URLs | `import x from "https://evil.com/x"` ❌ blocked | ✅ blocked |
| Relative paths `./` `../` | ❌ blocked — use `@/` | ✅ blocked |
| fetch() domain allowlist | Control which domains page JS can call | ✅ |

---

### 2.6 Tailwind

| Rule | Detail | Status |
|------|--------|--------|
| Standard Tailwind class syntax | `className="flex items-center gap-2 text-sm"` | ✅ |
| Dynamic classes via `tw-variants` attribute | Declare dynamic class strings for compiler to pick up | ✅ |
| Per-page CSS compilation | Each page compiles only its own classes | ✅ |
| Custom CSS in `styles/main.css` | Global styles via `zf-*` class convention | ✅ |
| Tailwind plugins | Extended utilities (e.g. scrollbar) | 🔮 |

**`tw-variants` usage:**
```tsx
<div
  tw-variants="bg-red-500 bg-green-500 bg-yellow-500"
  className={`bg-${status}-500`}
/>
```
Tells the compiler to include those classes even though they're assembled dynamically.

---

### 2.7 Server / Client Contract

| Rule | Detail | Status |
|------|--------|--------|
| `export const page = {}` | Page config (head, body, navigation mode) | ✅ |
| `export default function Page(props)` | Props come from server render vars | ✅ |
| SSR-first — all pages render on server | First response is full HTML, SEO-friendly | ✅ |
| Hydration payload via `#__rwe_payload` | JSON injected into page for client hydration | ✅ |
| `server` / `client` script namespace | Explicit server data vs client state split | 🔮 M2 |
| `expose` list for hydration payload | Control what server data reaches the client | 🔮 M2 |

---

### 2.8 Hydration

| Rule | Detail | Status |
|------|--------|--------|
| Full page hydration (default) | Entire `#__rwe_root` hydrated on load | ✅ |
| `hydrate="off"` — pure SSR block | No JS cost for static regions | 🔮 M4 |
| `hydrate="visible"` — lazy on scroll | Mount only when element enters viewport | 🔮 M4 |
| `hydrate="interaction"` — on first touch | Mount on first click/keypress | 🔮 M4 |
| `hydrate="idle"` — on browser idle | Mount during idle callback | 🔮 M4 |
| No extra `<script>` tags needed | Runtime baked into single inline module | ✅ |

---

### 2.9 Page Config

Defined by exporting from the page file:

```ts
export const page = {
  head: {
    title: ctx?.seo?.title ?? "",           // JS expression — resolved at module eval time
    description: ctx?.seo?.description ?? "",
  },
  html: {
    lang: "en",
  },
  body: {
    className: "h-screen bg-slate-950 text-white font-sans",
  },
  render: "ssr",        // "ssr" | "ssg" (future) | "client" (future)
  navigation: "history" // "history" (SPA) | "document" (full reload)
};
```

`globalThis.ctx` is injected by the RWE engine **before** the module loads, so `export const page` expressions can reference `ctx` directly as standard JavaScript. The `build_document_shell()` function in `render.rs` reads the already-resolved config and generates the full `<!DOCTYPE html>` wrapper with `<meta charset>`, viewport, title, description, body class, and lang attribute.

| Config | Status |
|--------|--------|
| `render: "ssr"` | ✅ default |
| `render: "ssg"` | 🔮 M5 |
| `render: "client"` | 🔮 M5 |
| `navigation: "history"` | ✅ default (inline SPA router in every page) |
| `navigation: "document"` | ✅ fallback available |

---

### 2.10 JavaScript Sandbox

| Rule | Detail | Status |
|------|--------|--------|
| Embedded deno_core V8 | SSR runs in embedded V8, no external process — restricted by default | ✅ |
| Per-render timeout | Configurable `deno_timeout_ms` | ✅ |
| No arbitrary FS access in templates | Deno permissions restrict file system | ✅ |
| No arbitrary net access in templates | fetch() to unlisted domains blocked at compile time | ✅ |
| `eval()` blocked | Security scanner rejects eval | ✅ |
| Direct DOM manipulation blocked | `document.querySelector` etc. forbidden in TSX | ✅ enforced by convention |

---

## 3. Folder Convention

```
templates/
  pages/           ← Entry points (one per route)
  components/
    ui/            ← Reusable UI components (design system)
    layout/        ← Shell/layout wrappers (wrap pages)
    behavior/      ← Pure TS behavior files (no JSX, DOM wiring)
  styles/
    main.css       ← Global CSS, zf-* custom classes
```

### Rules

| Rule | Detail |
|------|--------|
| Pages are entry points | One `pages/*.tsx` per route. Defines the page root. |
| UI components are stateless-first | Prefer pure render — no behavior wiring |
| Layout components wrap pages | `project-studio-shell.tsx` wraps all admin pages |
| Behavior files are pure `.ts` | No JSX, no render(). Wire DOM events, export init functions. |
| Shared reactive state | Goes in layout (entry page context) via `usePageState` |
| Components that need hooks | Can live anywhere — `"rwe"` import is a compile-time signal, stripped during bundling |
| Module-scope constants are auto-scoped | The bundler auto-prefixes `UPPER_SNAKE_CASE` constants per component (`__c0_VARIANT_CLASSES`, `__c1_VARIANT_CLASSES`) using word-boundary replacement. Developers write clean names — no manual prefixing needed. |

---

## 4. Build History

| # | Task | Status |
|---|------|--------|
| 1 | Fix `"rwe"` import in ALL files — shim via `prepare_template_root()` | ✅ Done |
| 2 | `useNavigate()` hook — SSR no-op, client uses `rwe_router.js` | ✅ Done |
| 3 | `<Link>` component — SSR `<a>`, client intercepts click via history API | ✅ Done |
| 4 | `fetch()` domain allowlist in `security::analyze()` | ✅ Done |
| 5 | Inline SPA router in `build_client_module()` — `history.pushState`, fetch, DOM swap, `popstate` | ✅ Done |
| 6 | CSS swap on navigation — `data-rwe-page-css` attribute + `[data-rwe-page-css]` swap in router | ✅ Done |
| 7 | Progress bar — `#__rwe_nav_bar`, `--rwe-nav-color` CSS variable, `rwe:nav-start` event | ✅ Done |
| 8 | Showcase demo: `/` (all hooks), `/blog` (search+filter), `/todo` (full app) at port 8787 | ✅ Done |

---

## 5. Navigation Loading Indicator

The inline SPA router ships a thin progress bar by default. Three tiers of customisation — no API changes needed between tiers.

### Tier 1 — CSS only (zero JS)

```css
/* hide entirely */
#__rwe_nav_bar { display: none; }

/* recolor */
:root { --rwe-nav-color: #6366f1; }

/* thicker */
#__rwe_nav_bar { height: 4px !important; }
```

### Tier 2 — `window.rweNavProgress` hook

Define this **before** the RWE module script executes. The default bar is never created; your callbacks are called instead.

```js
window.rweNavProgress = {
  start() { /* show spinner, skeleton, overlay */ },
  done()  { /* hide */  },
  fail()  { /* hide + optional error state */ },
};
```

### Tier 3 — Events (passive, no takeover)

Both events fire regardless of which tier is active.

| Event | When fired | `detail` |
|-------|-----------|---------|
| `rwe:nav-start` | Immediately before `fetch()` | `{ url }` |
| `rwe:nav` | After DOM swap + re-hydration | `{ url }` |

---

## 6. Non-Goals

| Item | Reason |
|------|--------|
| Full React compatibility layer | We use Preact — 99% compatible, not 100% by design |
| Generic VDOM reconciler | Preact handles this |
| Plugin API | Too early — core model not stable yet |
| Static site export (`build --ssg`) | Later milestone — don't confuse with SSR runtime |
| Class components | Functional only |
| `npm:react` or `npm:react-dom` imports | Forbidden — use `"rwe"` |
