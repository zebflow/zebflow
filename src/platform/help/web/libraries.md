# Zeb Libraries (`zeb/*`)

Bundled add-on libraries for Zebflow templates. Enable under **Settings → Libraries** before use.

Each library provides a pre-built JavaScript bundle served from `/assets/libraries/zeb/{lib}/{version}/`.

Most browser-only libraries should be loaded via dynamic `import()` inside `useEffect()`.
`zeb/icons` is the exception: import it explicitly at module top-level with `import "zeb/icons";`.


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

## zeb/livegeo — Live Map Hooks

Frontend-only hooks for playback, smoothing, and live track interactivity.

| Hook | Signature | Description |
|------|-----------|-------------|
| `usePlayback` | `({ start, end, initialTime, autoplay, speed, msPerSecond, loop })` | Timeline playback state with play, pause, reset, and seek. |
| `useTrackPlayback` | `(tracks, options?)` | Produces interpolated moving entity snapshots from timed track points. |
| `useTrackSmoothing` | `(target, options?)` | Smooths moving position and bearing updates. |
| `useMapFollow` | `(target, options?)` | Keeps map view state centered on a moving target. |

Use `Tool.geo` for cross-runtime math like `routeProgress`, `interpolateRoute`, and `bearing`.

---

## zeb/icons — SVG Icon Components

Lucide icon components. After enabling, you must still import the bundle explicitly:

```tsx
import "zeb/icons";

// Then use the icon components directly
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
  import('/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs')
    .then(({ EditorView, presets }) => {
      const view = new EditorView({
        extensions: presets.zebflow({ kind: "typescript" }),
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

## zeb/deckgl — Geospatial Map Visualization

WebGL-accelerated maps and data layers. Deck.gl 9.x bundled offline — ScatterplotLayer,
PathLayer, HeatmapLayer, GeoJsonLayer, TileLayer, ColumnLayer, and 20+ more layer types.

```tsx
import DeckMap from "zeb/deckgl";

<DeckMap
  height="500px"
  initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 12 }}
  layers={[{
    type: "ScatterplotLayer",
    data: input.locations,
    getPosition: "[longitude, latitude]",
    getFillColor: [0, 180, 255],
    getRadius: 50,
    pickable: true,
  }]}
  tooltip={true}
/>
```

Includes utility functions: `haversine`, `bearing`, `colorRamp`, `interpolateAlongPath`, `createAnimationLoop`.

See `help("web/deckgl")` for full documentation — all layer types, patterns (API-first,
WebSocket real-time, animation/playback, heatmaps, GeoJSON, tile basemaps), and pipeline examples.

---

## zeb/pdf — Client-side PDF Generation

Zero-dependency PDF 1.7 generator. Runs entirely in the browser — no server round-trip.

```tsx
import { createDocument, createTable, PAGE_SIZES } from "zeb/pdf";
```

### Exports

```ts
import {
  createDocument,   // DocumentBuilder factory — start here
  createTable,      // standalone table node helper
  PAGE_SIZES,       // { A4, A3, Letter, ... } — [width, height] in pts
  NODE_TYPES,       // enum of IR node kind strings
  render,           // async render(doc) → ArrayBuffer
  renderSync,       // sync renderSync(doc) → ArrayBuffer
  // Primitive IR node constructors (low-level):
  page, text, image, line, rect, table,
} from "zeb/pdf";
```

### Coordinate system

- **Y = 0 at bottom-left**, Y increases upward (standard PDF)
- A4 = 595 × 842 pts
- Content flows from **high Y → low Y** (top of page → bottom)
- `margin.top = 60` means content area top = `H - 60`

### DocumentBuilder API

```ts
const doc = createDocument({
  meta: { title, author, subject, creator },
  styles: { /* CSS-like class map */ },
  settings: {
    margin: { top: 60, right: 48, bottom: 72, left: 48 },
  },
});

// Add a page (first page is created automatically or explicitly)
const page1 = doc.page({
  size: "A4",                          // or [595, 842] or "Letter"
  margin: { top: 241, right: 48, bottom: 72, left: 48 },
  footer: { template: "Page {page} of {total}", align: "center" },
});

// Draw primitives on a specific page (absolute coordinates)
page1.rect({ x, y, width, height, fill, stroke, strokeWidth });
page1.line({ x1, y1, x2, y2, width, color });
page1.text("content", { x, y, style: { "font-size": 12, color: "#000" } });

// Flow content across pages automatically (auto page-break)
doc.tableFlow(tableNode, { pageOptions: contPageOpts });
doc.textFlow(textNode,   { pageOptions: contPageOpts });

// Produce output
const arrayBuffer = doc.toArrayBuffer();  // or renderSync(doc)
const blob        = doc.toBlob();         // Blob for URL.createObjectURL
```

### Page 1 header + info box pattern

The most common layout problem: manually-drawn header/info box overlapping with `tableFlow` content.

**Root cause**: `doc.tableFlow` starts at the page's content area top (`H - margin.top`). If you draw
boxes below the header with `page1.rect(...)`, they land in the content area and tables overwrite them.

**Fix**: make `margin.top` for page1 large enough to cover header + info box + gap. Keep a separate
`HDR_Y = H - HEADER_HEIGHT` for positioning header content (logo, text, etc.).

```ts
const H = 842, MT_HEADER = 155;
// MT_P1 = header height + info box area (62) + gaps (24)
const MT_P1 = MT_HEADER + 86; // 241

const page1 = doc.page({
  size: "A4",
  margin: { top: MT_P1, right: MR, bottom: MB, left: ML },
  footer: { ... },
});

// NAVY background covers the full page-1 top margin
page1.rect({ x: 0, y: H - MT_P1, width: W, height: MT_P1, fill: "#1a1a2e" });

// Header content positioned at HDR_Y = H - MT_HEADER (not H - MT_P1)
const HDR_Y = H - MT_HEADER; // 687
page1.text("University Name", { x: 80, y: HDR_Y + 106, style: { ... } });

// Info box drawn just below header band — it's now inside the margin, safe
const IY = HDR_Y - 14;
page1.rect({ x: ML, y: IY - 62, width: CW, height: 64, fill: "#f5f0e8" });
// ... text on info box ...

// tableFlow starts at H - MT_P1 = 601 — below the info box. No overlap.
doc.tableFlow(semesterTable, { pageOptions: contPageOpts });
```

### Bottom margin + footer clearance

`margin.bottom` is the distance from page bottom where content stops. The footer is drawn inside
this margin. Use at least **72pt** to avoid content clipping into footer text:

```ts
const MB = 72; // was 60 — the extra 12pt prevents last-row / footer collision
```

### Continuation pages

```ts
const contPageOpts = {
  size: "A4",
  margin: { top: 55, right: MR, bottom: MB, left: ML },
  footer: { template: "Page {page} of {total}  ·  MY DOCUMENT", align: "center" },
};
// Pass as second arg to tableFlow / textFlow:
doc.tableFlow(node, { pageOptions: contPageOpts });
```

### Table node structure

```ts
const tableNode = {
  _node: {
    type: "table",
    className: "my-table",
    columnWidths: [200, 100, 80],          // pt, must sum to content width
    columnAligns: ["left", "center", "right"],
    style: {},
    header: {                              // optional sticky header
      type: "row",
      className: "header",
      cells: [{ type: "cell", className: "cell", value: "Col A" }, ...],
    },
    body: [
      {
        type: "row",
        className: "row",
        style: { height: 18 },             // optional fixed row height
        cells: [{ type: "cell", className: "cell", value: "data" }, ...],
      },
    ],
  },
};
```

### Minimal complete example

```tsx
import { useState, useCallback } from "zeb";
import { createDocument } from "zeb/pdf";

export default function PdfPage() {
  const [url, setUrl] = useState<string | null>(null);

  const generate = useCallback(() => {
    const doc = createDocument({
      meta: { title: "My Doc" },
      styles: {
        ".cell": { padding: [3, 6, 3, 6], "font-size": 9 },
        ".header": { "background-color": "#1a1a2e", color: "#fff", "font-weight": "bold", "font-size": 9 },
        ".row": { "background-color": "#fff" },
      },
      settings: { margin: { top: 60, right: 48, bottom: 72, left: 48 } },
    });

    doc.tableFlow({
      _node: {
        type: "table",
        className: "",
        columnWidths: [200, 100, 100],
        columnAligns: ["left", "center", "right"],
        style: {},
        header: {
          type: "row", className: "header",
          cells: [
            { type: "cell", className: "cell", value: "Name" },
            { type: "cell", className: "cell", value: "Score" },
            { type: "cell", className: "cell", value: "Grade" },
          ],
        },
        body: [
          { type: "row", className: "row",
            cells: [
              { type: "cell", className: "cell", value: "Alice" },
              { type: "cell", className: "cell", value: "92" },
              { type: "cell", className: "cell", value: "A" },
            ] },
        ],
      },
    });

    setUrl(URL.createObjectURL(doc.toBlob()));
  }, []);

  return (
    <div>
      <button onClick={generate}>Generate PDF</button>
      {url && <iframe src={url} style={{ width: "100%", height: "600px" }} />}
    </div>
  );
}
```

---

## Critical rule: load browser-only bundles inside `useEffect`

For browser-only bundles like `zeb/d3`, `zeb/codemirror`, `zeb/markdown`, and `zeb/threejs`, never import the raw runtime asset at module top-level — they reference browser APIs unavailable during SSR.

`zeb/icons` does not use this raw runtime-asset pattern. Use `import "zeb/icons";` instead.

```tsx
// ✗ WRONG — crashes SSR
import * as d3 from '/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs';

// ✓ CORRECT — runs after mount, browser only
useEffect(() => {
  import('/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs')
    .then(({ d3 }) => { /* ... */ });
}, []);
```
