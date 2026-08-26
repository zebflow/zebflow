# zeb/prosemirror

ProseMirror 1.41 rich text editor for RWE templates. Fully offline — no CDN
fetches at runtime. All nine ProseMirror packages bundled inline via esbuild.

## Import

```tsx
import ProseEditor from "zeb/prosemirror";
```

The import line is stripped by the RWE compiler. `ProseEditor` is made
available as a global by the client preamble via `Object.assign(globalThis, bundle)`.

---

## `ProseEditor` Component

The primary way to use this library. Renders an editor in place with a managed
lifecycle — mounts once, survives Preact re-renders, cleans up on unmount.

```tsx
<ProseEditor
  id="my-editor"
  content="<p>Initial HTML</p>"
  stateKey="body"
  statsKey="bodyStats"
  toolbar="full"
  toolbarMode="inline"
  placeholder="Start writing…"
  editable={true}
  autofocus={false}
  className="w-full min-h-[300px]"
/>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `id` | `string` | auto-generated | Container element id. Required for `window.__zebProse.get(id)`. |
| `content` | `string` | `""` | Initial HTML. Loaded once at mount. Use this for pre-fetched data or default content. |
| `stateKey` | `string` | — | Page state key for two-way reactive sync. Editor → state on every keystroke. State → editor when the key changes via `setCurrent()`. |
| `statsKey` | `string` | — | Page state key to write `{ words, chars, isDirty }` on every keystroke. Read-only. |
| `toolbar` | `"minimal"` \| `"basic"` \| `"full"` \| `string[]` \| `false` | `"basic"` | Toolbar preset or custom item array. `false` hides the toolbar entirely. |
| `toolbarMode` | `"inline"` \| `"bubble"` | `"inline"` | `"bubble"` shows a floating toolbar only on text selection (mobile-friendly). |
| `placeholder` | `string` | — | Placeholder text shown when the editor is empty. |
| `editable` | `boolean` | `true` | `false` renders a read-only view (no cursor, no toolbar). |
| `autofocus` | `boolean` | `false` | Focus the editor immediately on mount. |
| `className` | `string` | `"w-full min-h-[200px]"` | Tailwind classes applied to the editor container. |

### Toolbar presets

| Preset | Items |
|--------|-------|
| `"minimal"` | bold, italic, strikethrough |
| `"basic"` | bold, italic, strike + h1, h2 + lists + blockquote |
| `"full"` | everything: all marks, headings, lists, blockquote, code blocks, hr, link, doodle, photo, undo, redo |

Custom array — use command names, plugin ids, or `"|"` for separators:

```tsx
<ProseEditor toolbar={["bold", "italic", "|", "h1", "h2", "|", "link", "ai-improve"]} />
```

---

## Content initialisation — `content` vs `stateKey`

> **Critical**: `window.__rwePageState[key]` is only written after the first
> Preact render commit (inside a `useEffect`). If you rely solely on `stateKey`
> for initial content, the editor will start empty.
>
> **Always pass `content={value}` when the initial value is known at render time.**

### Pattern: editor starts empty, API fills it later (800ms+)

The editor mounts empty. When the fetch resolves, `setBody(data.html)` fires
`rwe:state:change` → editor updates reactively. The editor will have finished
mounting by then, so the stateKey listener catches it.

```tsx
const [body, setBody] = usePageState("body", "");

useEffect(() => {
  fetch("/api/post/1").then(r => r.json()).then(d => setBody(d.html));
}, []);

<ProseEditor stateKey="body" toolbar="full" placeholder="Loading…" />
```

### Pattern: initial content known at render time

Pass `content` so it's baked into `data-config` before mount. `stateKey`
handles all subsequent reactive updates.

```tsx
const [body, setBody] = usePageState("body", existingPost.html);

<ProseEditor content={body} stateKey="body" toolbar="full" />
```

### Pattern: examiner swapper (single editor, many entries)

One read-only editor. `Prev`/`Next` buttons call `setCurrent(entry.html)`.
The editor swaps content via the `stateKey` listener — no remount, no flicker.
Pass `content={current}` so the first entry appears on load.

```tsx
const [current, setCurrent] = usePageState("current", entries[0].html);
const [idx, setIdx] = useState(0);

function go(i) { setIdx(i); setCurrent(entries[i].html); }

<ProseEditor
  id="examiner"
  content={current}
  stateKey="current"
  editable={false}
  toolbar={false}
  className="w-full min-h-[240px]"
/>
```

### Pattern: real-time edit + preview split

Two editors share a `stateKey`. The editable one pushes state on every
keystroke; the read-only one pulls it via `rwe:state:change`.

```tsx
const [synced, setSynced] = usePageState("synced", defaultHtml);

{/* Left: editable */}
<ProseEditor id="edit" content={synced} stateKey="synced" toolbar="basic" />

{/* Right: read-only live preview */}
<ProseEditor id="preview" content={synced} stateKey="synced" editable={false} toolbar={false} />
```

### Pattern: multi-editor per-question assessment

Each question gets its own editor and its own `stateKey`. On submit, collect
all values from page state.

```tsx
{questions.map(q => (
  <ProseEditor
    key={q.id}
    id={`ans-${q.id}`}
    stateKey={`ans_${q.id}`}
    statsKey={`stats_${q.id}`}
    toolbar="basic"
    placeholder="Write your answer…"
  />
))}

// On submit — read from page state:
const answers = questions.map(q => ({
  id: q.id,
  html: window.__rwePageState?.[`ans_${q.id}`] ?? "",
}));
// Or imperatively:
const html = window.__zebProse.get(`ans-${q.id}`).getHTML();
```

### Pattern: read-only list rendered from API data

Editors added via Preact re-renders (e.g. after a fetch) are auto-mounted by
the MutationObserver — no explicit `mountProseEditor` call needed.

```tsx
const [submissions, setSubmissions] = useState([]);
useEffect(() => { fetch("/api/subs").then(r => r.json()).then(setSubmissions); }, []);

{submissions.map(s => (
  <ProseEditor
    key={s.id}
    id={`sub-${s.id}`}
    content={s.html}
    editable={false}
    toolbar={false}
    className="w-full min-h-[120px]"
  />
))}
```

---

## `statsKey` — word and character counts

```tsx
const [stats] = usePageState("bodyStats", { words: 0, chars: 0, isDirty: false });

<ProseEditor stateKey="body" statsKey="bodyStats" toolbar="full" />

<p>{stats.words} words · {stats.chars} chars {stats.isDirty && "· unsaved"}</p>
```

The stats object shape:
```ts
{ words: number; chars: number; isDirty: boolean }
```

---

## Events

Both events bubble from the editor container.

### `zeb:prose:change`

Fires on every document change.

```tsx
// From a behavior file (via useEffect in the page or after mount):
document.getElementById("my-editor").addEventListener("zeb:prose:change", (e) => {
  console.log(e.detail.html);    // current HTML string
  console.log(e.detail.json);    // ProseMirror JSON doc
  console.log(e.detail.words);   // word count
  console.log(e.detail.chars);   // char count
  console.log(e.detail.isDirty); // true after first change
});
```

### `zeb:prose:ready`

Fires once when the editor finishes mounting. Use this to get the instance
synchronously without polling.

```tsx
document.getElementById("my-editor").addEventListener("zeb:prose:ready", (e) => {
  const inst = e.detail.instance;
  inst.setHTML("<p>Injected after mount</p>");
});
```

---

## Imperative API — `window.__zebProse`

```ts
const inst = window.__zebProse.get("my-editor");  // returns ProseInstance | undefined

inst.getHTML()           // → string — current editor content as HTML
inst.getJSON()           // → object — ProseMirror document JSON
inst.setHTML("<p>…</p>") // replace all content
inst.setJSON(json)       // replace all content from a PM JSON doc
inst.focus()             // focus the editor
inst.blur()              // blur the editor
inst.destroy()           // destroy editor, clean up, remove from registry
```

`get()` returns `undefined` if the editor hasn't mounted yet. Listen for
`zeb:prose:ready` to guarantee availability.

---

## Custom plugins

Register via `window.__zebProse.registerPlugin()`. Plugins are available
instantly on all editors on the page (including existing ones on the next
toolbar rebuild).

```tsx
// In a behavior file, or inside useEffect in the page:
useEffect(() => {
  const tryRegister = () => {
    if (!window.__zebProse) { setTimeout(tryRegister, 100); return; }
    window.__zebProse.registerPlugin({
      id:    "ai-rewrite",
      label: "AI",
      icon:  `<svg>…</svg>`,   // any SVG or HTML string
      async onActivate(ctx) {
        const text = ctx.getSelectedText();
        if (!text) return;
        ctx.setLoading(true, ctx._btnEl);  // spins the toolbar button
        const improved = await callAiApi(text);
        ctx.replaceSelection(improved);
        ctx.setLoading(false, ctx._btnEl);
      },
    });
  };
  tryRegister();
}, []);

// Then use "ai-rewrite" in any toolbar:
<ProseEditor toolbar={["bold", "italic", "|", "ai-rewrite"]} stateKey="body" />
```

### Plugin context (`ctx`)

| Method | Description |
|--------|-------------|
| `ctx.getHTML()` | Current editor content as HTML |
| `ctx.getJSON()` | Current content as PM JSON |
| `ctx.getSelectedText()` | Plain text of the current selection |
| `ctx.insertHTML(html)` | Insert HTML at cursor position |
| `ctx.replaceSelection(html)` | Replace current selection with HTML |
| `ctx.insertImage(src, alt)` | Insert `<img src="…">` at cursor |
| `ctx.openOverlay(el)` | Append an overlay element to body; returns `close()` |
| `ctx.setLoading(bool, btnEl)` | Toggle loading spinner on the toolbar button |
| `ctx.view` | Raw ProseMirror `EditorView` for advanced use |

---

## Direct mount — `mountProseEditor`

For cases where you manage the lifecycle yourself (no component, no
MutationObserver). Useful in behavior files.

```tsx
import { mountProseEditor } from "zeb/prosemirror";

const container = document.getElementById("my-div");
const inst = await mountProseEditor(container, {
  content:     "<p>Hello</p>",
  stateKey:    "body",
  toolbar:     "full",
  editable:    true,
  placeholder: "Write something…",
});

inst.getHTML(); // → "<p>Hello</p>"
```

---

## CSS customisation

The bundle injects a single `<style data-zeb-prose>` tag. It uses CSS custom
properties from the zebflow design token set with dark-theme fallbacks.
Override any variable on the container or a parent element:

```css
/* Custom accent colour for one editor */
#my-editor {
  --border:           #4f46e5;
  --accent:           260 90% 30%;
  --accent-foreground: 260 90% 98%;
  --muted:            260 30% 10%;
  --foreground:       #f8fafc;
}
```

| Variable | Used for |
|----------|----------|
| `--border` | Toolbar bottom border, separator lines, blockquote, hr |
| `--muted` | Toolbar background, code block background |
| `--muted-foreground` | Toolbar button icons, placeholder text, blockquote text |
| `--foreground` | Editor text colour |
| `--accent` | Button hover/active background |
| `--accent-foreground` | Button hover/active text colour |
| `--background` | Bubble toolbar background |

---

## Bundle details

| Property | Value |
|----------|-------|
| Package | `prosemirror-view@1.41.6` + 8 sibling PM packages |
| Bundle | `runtime/prosemirror.bundle.mjs` (~243 KB minified) |
| CDN fetches at runtime | **None** — fully offline |
| Build tool | esbuild |

### Rebuild the bundle

```sh
cd /tmp/zeb-pm-build
# entry.mjs and package.json are in libraries/zeb/prosemirror/0.1/runtime/
cp libraries/zeb/prosemirror/0.1/runtime/entry.mjs .
cp libraries/zeb/prosemirror/0.1/runtime/package.json .
npm install
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --minify \
  --outfile=prosemirror.bundle.mjs
cp prosemirror.bundle.mjs libraries/zeb/prosemirror/0.1/runtime/
cargo build
```
