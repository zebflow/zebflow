# Zeb Libraries (`zeb/*`)

Bundled add-on libraries for Zebflow templates. Enable under **Settings → Libraries** before use.

Each library provides a pre-built JavaScript bundle served from `/assets/libraries/zeb/{lib}/{version}/`. Load them via `useEffect` + dynamic `import()` — **never at module top-level** (breaks SSR).

---

## zeb/use — Utility Hooks

Extra hooks beyond the core globals. After enabling, all hooks become globals — no import needed.

| Hook | Signature | Description |
|------|-----------|-------------|
| `useDebounce` | `(value, delay)` | Debounced state value. Updates after `delay` ms of inactivity. |
| `useThrottle` | `(value, delay)` | Throttled state value. Updates at most once per `delay` ms. |
| `useLocalStorage` | `(key, initial)` | `localStorage`-backed state. Returns `[value, setter]`. |
| `useClipboard` | `()` | Copy to clipboard with auto-reset. Returns `{ copied, copy(text) }`. |
| `useTemporaryState` | `(initial, duration)` | State that auto-resets to `initial` after `duration` ms. |
| `useWindowEvent` | `(event, handler)` | `window.addEventListener` with auto cleanup on unmount. |
| `useLazyModule` | `(importFn)` | Dynamic import with loading state. Returns `[module, loading, error]`. |
| `useSearchParams` | `()` | Read/write URL search params via `history.replaceState`. Returns `[URLSearchParams, setter]`. |
| `useSplitPane` | `(options?)` | Pointer-drag resizable split pane. Attach returned ref to container. |
| `useClickAway` | `(handler)` | Fires handler on click/touch outside the returned ref element. |
| `useInterval` | `(fn, delay)` | `setInterval` with cleanup. Pass `null` to pause. |
| `useGeolocation` | `()` | Device position watcher. Returns `{ loading, error, coords }`. |
| `useTree` | `(options?)` | Tree expansion state. Returns `{ expanded, isExpanded, toggle, expand, collapse, expandAll, collapseAll }`. |

```tsx
// All hooks are globals after enabling zeb/use — no import needed
const [search, setSearch] = useState("");
const debouncedSearch = useDebounce(search, 300);

const { copy, copied } = useClipboard();
<button onClick={() => copy(state.apiKey)}>{copied ? "Copied!" : "Copy"}</button>

const [params, setParams] = useSearchParams();
const page = Number(params.get("page") ?? "1");
```

---

## zeb/icons — SVG Icon Components

Lucide icon components. After enabling, all icons are available as globals — no import needed.

```tsx
// Globals — use directly
<Search className="w-4 h-4" />
<Loader2 className="w-4 h-4 animate-spin text-accent" />
<Trash2 className="w-4 h-4 text-red-400" />
<CheckCircle className="w-4 h-4 text-green-400" />
```

All accept `className` and `size` (number, defaults to 16) props.

**Available icons:**

- **Navigation:** `ChevronLeft`, `ChevronRight`, `ChevronDown`, `ChevronUp`, `ChevronsLeft`, `ChevronsRight`, `ChevronsUpDown`, `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`
- **Actions:** `Plus`, `Minus`, `X`, `Check`, `Pencil`, `Trash2`, `Copy`, `Clipboard`, `Save`, `Download`, `Upload`, `ExternalLink`, `Undo2`, `Redo2`, `RefreshCw`, `Search`, `Filter`
- **Status:** `AlertCircle`, `AlertTriangle`, `Info`, `CheckCircle`, `CheckCircle2`, `XCircle`, `Loader2`
- **UI chrome:** `Eye`, `EyeOff`, `Lock`, `Unlock`, `Settings`, `Menu`, `MoreHorizontal`, `MoreVertical`, `Maximize2`, `Minimize2`, `PanelLeft`, `PanelRight`, `SidebarOpen`, `SidebarClose`, `Bell`, `BellOff`
- **Data/Dev:** `Database`, `TableIcon`, `BarChart2`, `PieChart`, `TrendingUp`, `TrendingDown`, `Columns2`, `Code2`, `Terminal`, `Cpu`, `Cloud`, `Wifi`
- **Files:** `File`, `FileText`, `Folder`, `FolderOpen`
- **People:** `User`, `Users`, `KeyRound`, `LogIn`, `LogOut`
- **Misc:** `Globe`, `Package`, `Zap`, `Star`, `Layers`, `LayoutGrid`, `ListIcon`, `Tag`

---

## zeb/markdown — Markdown Rendering

Renders markdown to HTML with sanitisation (marked + DOMPurify).

```tsx
// Markdown component — global after enabling
<Markdown content={state.body} className="prose prose-invert" />
```

Or imperatively in `useEffect`:

```tsx
const containerRef = useRef(null);
useEffect(() => {
  import('/assets/libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs')
    .then(({ renderMarkdown }) => {
      if (containerRef.current) {
        containerRef.current.innerHTML = renderMarkdown(state.body ?? "");
      }
    });
}, [state.body]);
return <div ref={containerRef} />;
```

---

## zeb/codemirror — Code Editor

Full CodeMirror 6 editor. Always load in `useEffect` (browser only).

```tsx
const editorRef = useRef(null);

useEffect(() => {
  import('/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs')
    .then(({ EditorView, basicSetup, EditorState }) => {
      const view = new EditorView({
        extensions: [basicSetup],
        parent: editorRef.current,
        doc: state.code ?? "",
      });
      // save handle if needed
    });
}, []);

return <div ref={editorRef} className="h-64 border border-border rounded overflow-hidden" />;
```

The `CodeEditor` Preact wrapper component lives in `@/components/ui/code-editor` (platform studio only).

---

## zeb/prosemirror — Rich Text Editor

ProseMirror WYSIWYG editor with toolbar, plugins, and a Preact wrapper.

```tsx
// ProseEditor component — available after enabling
<ProseEditor
  id="body-editor"
  stateKey="body"        // syncs to usePageState key "body"
  toolbar="basic"        // "minimal" | "basic" | "full" | false
  toolbarMode="inline"   // "inline" | "bubble"
  editable={true}
  placeholder="Start writing…"
/>
```

See also: `help_docs topic=zeb/prosemirror` for full config API and plugin docs.

---

## zeb/d3 — Data Visualisations

Full d3 v7 namespace plus a `useD3` Preact hook.

```tsx
const chartRef = useRef(null);

useEffect(() => {
  import('/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs')
    .then(({ d3 }) => {
      const svg = d3.select(chartRef.current)
        .append("svg")
        .attr("width", 500).attr("height", 300);

      // Draw bars, axes, etc.
      const x = d3.scaleBand().domain(state.labels).range([0, 500]).padding(0.2);
      // ...
    });
}, [state.data]);

return <div ref={chartRef} />;
```

`useD3(callback, deps)` hook (from zeb/d3) provides a more idiomatic way:

```tsx
const ref = useD3((container, d3) => {
  const svg = d3.select(container).append("svg");
  // ...
}, [state.data]);
return <div ref={ref} />;
```

---

## zeb/threejs — 3D Scenes

Three.js with scene helpers.

```tsx
const canvasRef = useRef(null);

useEffect(() => {
  import('/assets/libraries/zeb/threejs/0.1/runtime/threejs.bundle.mjs')
    .then(({ mountThreeScene }) => {
      mountThreeScene(canvasRef.current, {
        // scene setup config
      });
    });
}, []);

return <div ref={canvasRef} className="w-full h-96" />;
```

---

## zeb/pdf — PDF Generation (pipeline-side)

Used in pipelines via `n.pdf.generate`. No direct frontend component.

---

## Critical rule: always load inside `useEffect`

Never import `zeb/*` bundles at module top-level — they reference browser APIs unavailable during SSR:

```tsx
// ✗ WRONG — crashes SSR
import * as d3 from '/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs';

// ✓ CORRECT — runs after mount, browser only
useEffect(() => {
  import('/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs')
    .then(({ d3 }) => { /* ... */ });
}, []);
```
