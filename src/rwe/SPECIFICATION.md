# RWE — Reactive Web Engine Specification

> **Design principle:** A developer who knows React should be able to open an RWE file and feel at home.
> No new mental model. No custom directives. No magic strings. Just TSX, imports, and standard hooks.

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
│  SERVER (Rust + Deno)                                        │
│                                                              │
│  Request → compile() → render_ssr() → HTML string           │
│                                                              │
│  Deno runs ssr_worker.mjs (persistent process)              │
│  installGlobals() sets useState, useEffect, etc as globals  │
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
Source .tsx file
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
│ collect_imports  │  Gather all import sources from AST
│ validate_        │  Check against allowlist:
│   allowlist()    │    "rwe", "npm:*", "node:*", "jsr:*", "@/*"
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ strip_runtime_   │  Remove `import { ... } from "rwe"` lines
│   imports()      │  ⚠️  Currently only runs on entry page source
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ rewrite_imports  │  @/ → absolute temp dir path
│ (alias rewrite)  │  Applies to ALL files in temp dir
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ JSX_PRELUDE      │  Prepend `/** @jsxImportSource npm:preact */`
│  inject          │  Tells Deno to use preact for JSX transform
└────────┬─────────┘
         │
         ▼
   CompiledTemplate
   ├── server_module_source  (for Deno SSR)
   ├── client_module_source  (for browser hydration)
   └── imports, diagnostics, hydrate_mode
```

---

### 1.3 Render Pipeline

```
CompiledTemplate + vars (JSON)
        │
        ├──► render_ssr(server_module_source, vars)
        │         │
        │         ▼
        │    Deno persistent worker (ssr_worker.mjs)
        │    ├── installGlobals() — useState, useEffect,
        │    │                      useRef, useMemo,
        │    │                      usePageState, h, Fragment
        │    ├── dynamic import(server_module_source)
        │    │   → imports cascade through @/ resolved paths
        │    └── render Page component → HTML string
        │
        ├──► transpile_client_cached(client_module_source)
        │         │
        │         ▼
        │    Deno worker transpiles TSX → JS
        │    Result cached by source hash (in-memory, 256 cap)
        │
        └──► build_client_module(transpiled_js)
                  │
                  ▼
             Inline preamble injected (NO extra HTTP requests):
             ├── import preact + hooks from esm.sh (pinned 10.28.4)
             ├── globalThis.useState = useState
             ├── globalThis.useEffect = useEffect
             ├── globalThis.useRef = useRef
             ├── globalThis.useMemo = useMemo
             ├── globalThis.usePageState = __rweUsePageState
             ├── globalThis.useNavigate (checks window.rweNavigate)
             ├── globalThis.Link (intercepts click → window.rweNavigate)
             ├── globalThis.h = h
             ├── globalThis.React = { createElement: h, Fragment }
             ├── window.rweNavigate — inline SPA router (if not already defined)
             │     fetch → DOMParser → #__rwe_root swap + [data-rwe-page-css] swap
             │     history.pushState, popstate handler, rwe:nav-start + rwe:nav events
             │     Progress bar (#__rwe_nav_bar, --rwe-nav-color CSS variable)
             │     Admin shell's rwe_router.js takes precedence when present
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

### 1.5 Deno Worker

```
static WORKER: LazyLock<Mutex<DenoWorker>>
```

- **Single persistent process** — module cache lives for the server's lifetime
- Restart server = clear module cache (important after file changes in dev)
- Handles both SSR render and client transpile
- Auto-respawns on crash

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
| `useRef` | Mutable ref to DOM element | ✅ |
| `useMemo` | Memoized computed value | ✅ |
| `usePageState` | Shared state across all components on the same page | ✅ |
| `useNavigate` | SPA navigation hook — `const nav = useNavigate(); nav("/path")` | ✅ |
| `Link` | Router-aware anchor — `<Link href="/path">Go</Link>` | ✅ |

**`"rwe"` import in component files — resolved:**
`prepare_template_root()` in `src/rwe/core/mod.rs` writes `rwe.ts` shim and rewrites `from "rwe"` → absolute shim path in **every** `.tsx/.ts/.jsx/.js` file in the template root. The shim re-exports all globals from `globalThis`. Works in pages, components, layouts — everywhere.

---

### 2.3 Navigation

| Rule | Detail | Status |
|------|--------|--------|
| SPA navigation via `useNavigate()` | `const nav = useNavigate(); nav("/projects/x")` | ✅ |
| `<Link href="...">` component | Intercepts click, uses history API | ✅ |
| Inline SPA router in every page | `window.rweNavigate` baked into `build_client_module()` — no extra file needed | ✅ |
| `history.pushState` URL updates | Browser URL bar reflects current page without reload | ✅ |
| Back/forward browser buttons | `popstate` handler re-runs `rweNavigate` | ✅ |
| CSS swap on navigation | `[data-rwe-page-css]` style tag swapped alongside `#__rwe_root` | ✅ |
| Page title update on navigation | `document.title` set from fetched page | ✅ |
| `rwe:nav-start` event | Dispatched before fetch begins — for custom loading indicators | ✅ |
| `rwe:nav` event | Dispatched after swap completes — for analytics, side effects | ✅ |
| Progress bar | Thin top bar, `#__rwe_nav_bar`, color via `--rwe-nav-color` CSS var | ✅ |
| Admin shell takes precedence | `rwe_router.js` defines `window.rweNavigate` first — inline router skips itself | ✅ |
| Direct `window.location.href` | ❌ Never — raw DOM access forbidden in components | ❌ omit |
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
| `export const app = {}` | Marks entry page (required for layout components) | ✅ |
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
// Page navigation + render mode (future full support)
export const page = {
  render: "ssr",        // "ssr" | "ssg" (future) | "client" (future)
  navigation: "history" // "history" (SPA) | "document" (full reload)
};
```

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
| Deno isolated worker | SSR runs in Deno, not Node — restricted by default | ✅ |
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
| Components that need hooks | Can live anywhere — `"rwe"` import works in all files via `prepare_template_root()` shim |

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
