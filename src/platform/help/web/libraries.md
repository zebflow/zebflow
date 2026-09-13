# `zeb/*` runtime libraries

Browser libraries the platform ships with the binary and serves from
`/assets/libraries/zeb/<lib>/0.1/runtime/…`. A page uses one with a **static
import**; the compiler recognises the specifier, the page loads the bundle
before hydrating, and the names arrive as globals in the browser. Nothing to
install, nothing to configure:

```tsx
import { d3 } from "zeb/d3";
import { DeckMap, ScatterplotLayer } from "zeb/deckgl";
import { renderMarkdown } from "zeb/markdown";
```

Dynamic `import()` is refused by the default security policy
(`RWE_SECURITY_DYNAMIC_IMPORT`), and importing a bundle URL directly is not
supported — always the `zeb/<lib>` specifier. The libraries are browser code:
call them from `useEffect`, event handlers or `useD3`-style hooks, not during
render. Components the libraries provide (`DeckMap`, `ThreeScene`, `CodeEditor`,
`Markdown`, `VrmViewer`) render a placeholder on the server and mount in the
browser, so they may appear in JSX directly.

A project can restrict which libraries its pages may load under
`rwe.libraries` in `zebflow.yaml` (`POST /api/projects/{o}/{p}/rwe/libraries/enable`);
with no list, any of them loads on demand.

| Library | Import | For |
|---|---|---|
| `zeb/use` | hooks | debounce, clipboard, localStorage, intervals, trees… |
| `zeb/livegeo` | hooks | playback and smoothing for moving things on a map |
| `zeb/markdown` | `renderMarkdown`, `marked`, `DOMPurify`, `<Markdown>` | Markdown → sanitised HTML |
| `zeb/codemirror` | `codemirror`, `presets`, `createZebflowEditorExtensions`, `<CodeEditor>` | a code editor (CodeMirror 6) |
| `zeb/prosemirror` | `createEditor`, `createSchema`, `htmlToDocument`, `EMPTY_DOC`, `pm` | the engine under `zeb/ui/editor` |
| `zeb/d3` | `d3`, `useD3` | charts and data visualisation (d3 v7) |
| `zeb/deckgl` | `DeckMap`, layer classes, `buildLayers`, `colorRamp`, … | WebGL maps |
| `zeb/threejs`, `zeb/threejs-vrm` | `threejs`, `<ThreeScene>`, `mountThreeScene`; `vrm`, `<VrmViewer>` | 3D scenes and VRM avatars |
| `zeb/graphui` | `createGraphUI`, `GraphStore`, … | node-graph editors |
| `zeb/pdf` | `createDocument`, `createTable`, `render`, `PAGE_SIZES`, … | PDF generation in the browser |

There is no icons library; use inline SVG (the `zeb/ui` components do).

---

## zeb/use — hooks

Import them from `"zeb/use"` in every file that uses them, next to the
`zeb/react` imports. On the server they return their initial value.

| Hook | Signature | Description |
|------|-----------|-------------|
| `useDebounce` | `(value, delay)` | Debounced state value. Updates after `delay` ms of inactivity. |
| `useThrottle` | `(value, delay)` | Throttled state value. Updates at most once per `delay` ms. |
| `useLocalStorage` | `(key, initial)` | `localStorage`-backed state. Returns `[value, setter]`. |
| `useClipboard` | `()` | Copy to clipboard with auto-reset. Returns `{ copied, copy(text) }`. |
| `useTemporaryState` | `(initial, duration)` | State that auto-resets to `initial` after `duration` ms. |
| `useWindowEvent` | `(event, handler)` | `window.addEventListener` with auto cleanup on unmount. |
| `useLazyModule` | `(importFn)` | Dynamic import with loading state. Returns `[module, loading, error]`. |
| `useSplitPane` | `(options?)` | Pointer-drag resizable split pane. Attach returned ref to container. |
| `useClickAway` | `(handler)` | Fires handler on click/touch outside the returned ref element. |
| `useInterval` | `(fn, delay)` | `setInterval` with cleanup. Pass `null` to pause. |
| `useGeolocation` | `()` | Device position watcher. Returns `{ loading, error, coords }`. |
| `useTree` | `(options?)` | Tree expansion state. Returns `{ expanded, isExpanded, toggle, expand, collapse, expandAll, collapseAll }`. |

```tsx
import { useState, useSearchParams } from "zeb/react";
import { useDebounce, useClipboard } from "zeb/use";

const [search, setSearch] = useState("");
const debounced = useDebounce(search, 300);
const { copy, copied } = useClipboard();
<button onClick={() => copy(apiKey)}>{copied ? "Copied" : "Copy"}</button>

const params = useSearchParams();                 // URL state is zeb/react's
const page = Number(params.get("page") ?? "1");
```

---

## zeb/livegeo — live map hooks

Browser-only hooks for playback, smoothing and following a moving target.

| Hook | Signature | Description |
|------|-----------|-------------|
| `usePlayback` | `({ start, end, initialTime, autoplay, speed, msPerSecond, loop })` | Timeline playback state with play, pause, reset, and seek. |
| `useTrackPlayback` | `(tracks, options?)` | Produces interpolated moving entity snapshots from timed track points. |
| `useTrackSmoothing` | `(target, options?)` | Smooths moving position and bearing updates. |
| `useMapFollow` | `(target, options?)` | Keeps map view state centered on a moving target. |

Use `Tool.geo` (`routeProgress`, `interpolateRoute`, `bearing`) for the maths
that must also run on the server. Guard these hooks behind `useEffect` or a
component rendered only in the browser — they have no server stubs.

---

## zeb/markdown

```tsx
import { Markdown, renderMarkdown } from "zeb/markdown";

<Markdown content={post.body_md} className="max-w-none" />
// or
<div dangerouslySetInnerHTML={{ __html: renderMarkdown(text) }} />   // refused: raw HTML is off by policy
```

`renderMarkdown(text)` returns sanitised HTML (marked + DOMPurify). `<Markdown>`
renders it into a `div` and exists only when the file imports it — it is not a
global. There is no `prose` stylesheet; style headings, lists and code with
your own CSS or wrap the output in a container with utilities. For rich text
authored in the platform, prefer the editor's JSON document and
`zeb/ui/editor-render` (`help("web/ui")`).

---

## zeb/codemirror

```tsx
import { useEffect, useRef } from "zeb/react";
import { codemirror, presets } from "zeb/codemirror";

export default function Editor({ code }) {
  const host = useRef(null);
  useEffect(() => {
    const view = new codemirror.EditorView({
      parent: host.current,
      doc: code ?? "",
      extensions: presets.zebflow({ kind: "typescript" }),
    });
    return () => view.destroy();
  }, []);
  return <div ref={host} className="h-64 overflow-hidden rounded-md border border-border" />;
}
```

`codemirror` is the CodeMirror 6 namespace (`EditorView`, `EditorState`,
`keymap`, languages…); `presets.zebflow({ kind })` is the platform's own
extension set; `<CodeEditor>` is the same thing as a component.

---

## zeb/prosemirror — the editing engine

ProseMirror, Zebflow's document schema, and `createEditor`. Engine only: it
draws no toolbar, menu or colour. For a page that just needs an editor, use
`zeb/ui/editor` — a Notion-style block editor built on this (see
`help("web/ui")`). Reach for the engine to build a custom one.

```tsx
import { useEffect, useRef } from "zeb/react";
import { createEditor, EMPTY_DOC } from "zeb/prosemirror";

const mountRef = useRef(null);
useEffect(() => {
  const editor = createEditor(mountRef.current, {
    doc: EMPTY_DOC,                       // a ProseMirror document (JSON)
    classes: { root: "outline-none", heading1: "text-3xl font-bold", … },
    placeholder: "Write…",
    onChange: (doc) => save(doc),         // JSON, every transaction that changes the document
    onSlash: (s) => {},                   // { query, from, to, left, top, bottom } or null — draw your own menu
    onSelection: (s) => {},               // { from, to, left, top, bottom, marks, block, attrs, text } or null
    uploadImage: async (file) => url,     // pasted or dropped images; omit to refuse them
  });
  return () => editor.destroy();
}, []);

editor.exec("heading", 2);  editor.exec("toggleBold");  editor.exec("list", "todo");
editor.exec("image", { src, alt });  editor.exec("setLink", href);  editor.getJSON();  editor.setJSON(doc);
```

Blocks: paragraph, heading 1–3, blockquote, callout, code_block, image,
bullet/ordered/todo lists, horizontal_rule. Marks: bold, italic, underline,
strike, code, link. Markdown shortcuts, Mod-key bindings, history, drop
cursor, a hover drag handle and to-do checkboxes are built in. Rendering a
stored document without the engine is `zeb/ui/editor-render`
(`DocumentView`, `renderDocumentHtml`, `documentText`). `pm` exports every
ProseMirror package for plugins of your own; `htmlToDocument(html)` converts
legacy HTML.

---

---

## zeb/d3

```tsx
import { useD3 } from "zeb/d3";

export default function Bars({ rows }) {
  const ref = useD3((container, d3) => {
    const svg = d3.select(container).selectAll("svg").data([null]).join("svg").attr("width", 480).attr("height", 240);
    const x = d3.scaleBand().domain(rows.map((r) => r.label)).range([0, 480]).padding(0.2);
    const y = d3.scaleLinear().domain([0, d3.max(rows, (r) => r.value)]).range([240, 0]);
    svg.selectAll("rect").data(rows).join("rect")
      .attr("x", (r) => x(r.label)).attr("y", (r) => y(r.value))
      .attr("width", x.bandwidth()).attr("height", (r) => 240 - y(r.value))
      .attr("class", "fill-chart-1");
  }, [rows]);
  return <div ref={ref} />;
}
```

`useD3(callback, deps)` returns a ref; the callback runs in the browser with
the container element and the full d3 v7 namespace. `import { d3 }` gives the
namespace directly for use inside `useEffect`. For a handful of bars or a
sparkline, plain SVG in JSX with theme classes needs no library at all.

---

## zeb/deckgl

```tsx
import { DeckMap } from "zeb/deckgl";

<DeckMap
  height="500px"
  initialViewState={{ longitude: 106.8, latitude: -6.2, zoom: 10 }}
  layers={[{ type: "ScatterplotLayer", data: input.rows, getPosition: "[lon, lat]", getFillColor: [0, 180, 255], getRadius: 50, pickable: true }]}
  tooltip
/>
```

Layer classes, `buildLayers`, `colorRamp`, `haversine`, `bearing`,
`interpolateAlongPath`, `createAnimationLoop`, the imperative
`mountDeckMap` / `createDeckMapRuntime`, WebSocket and playback patterns:
`help("web/deckgl")`.

---

## zeb/threejs and zeb/threejs-vrm

```tsx
import { ThreeScene } from "zeb/threejs";
import { VrmViewer } from "zeb/threejs-vrm";

<ThreeScene height="400px" config={{ background: "#0b0b0f", cameraZ: 4, fov: 60 }} />
<VrmViewer modelUrl="/files/o/p/public/avatar.vrm" height="480px" autoRotate />
```

`threejs` is the namespace; `ensureThree`, `createSceneRuntime` and
`mountThreeScene(host, options)` are the imperative forms. Read the library's
README under `blessed/rwe-libraries/threejs/` for the scene runtime options.

---

## zeb/graphui

`createGraphUI(root, options)` mounts a node-graph editor into a DOM element;
`GraphStore` and the graph classes are exported for custom tooling. Browser
only — mount in `useEffect`.

---

## zeb/pdf

A PDF 1.7 generator that runs in the browser — no server round-trip.

```tsx
import { createDocument, createTable, PAGE_SIZES } from "zeb/pdf";
```

Exports: `createDocument` (the builder — start here), `createTable`,
`PAGE_SIZES` (`A4`, `A3`, `Letter`, … as `[w, h]` in points), `NODE_TYPES`,
`render(doc)` / `renderSync(doc)` (an IR document → bytes), the IR node
constructors `page`, `text`, `image`, `line`, `rect`, `table`, plus
`PdfDocument`, `readPdf`, `DEFAULT_STYLES`, `parseColor`, `measureTextWidth`,
`wrapText`.

Coordinates: y = 0 at the bottom-left, increasing upward; A4 is 595 × 842 pt;
content flows from high y to low y; `margin.top = 60` puts the content top at
`H - 60`.

```ts
const doc = createDocument({
  meta: { title, author },
  styles: { ".cell": { padding: [3, 6, 3, 6], "font-size": 9 } },   // CSS-like class map
  settings: { margin: { top: 60, right: 48, bottom: 72, left: 48 } },
});

const page1 = doc.page({ size: "A4", footer: { template: "Page {page} of {total}", align: "center" } });
page1.rect({ x, y, width, height, fill, stroke, strokeWidth });
page1.line({ x1, y1, x2, y2, width, color });
page1.text("content", { x, y, style: { "font-size": 12, color: "#000" } });

doc.textFlow("a long paragraph…", { style: { "font-size": 10 }, pageOptions: contPageOpts });  // wraps and page-breaks
doc.tableFlow(tableNode, { pageOptions: contPageOpts });                                        // rows across pages

const bytes = doc.toBytes();          // Uint8Array
const blob  = doc.toBlob();           // application/pdf
const url   = doc.toUrl();            // object URL for an <iframe> or a download link
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
import { useState, useCallback } from "zeb/react";
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
