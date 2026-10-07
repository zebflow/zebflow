# Rich text: the editor and its extensions

`zeb/ui/editor` is the block editor; `zeb/ui/editor-render` renders what it
writes. A document is **ProseMirror JSON** — that is what a project stores and
the only source of truth. Public pages show it as **HTML rendered on the
server** from that JSON, on every request, with no browser and no editor
involved. Projects add their own blocks with **extensions**, and the same
extension list is given to the editor and to every renderer, so what was
written is what is shown.

```tsx
import { Editor } from "zeb/ui/editor";
import { DocumentView, renderDocumentHtml, documentText } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";

<Editor value={doc} onChange={setDoc} extensions={EXTENSIONS} uploadImage={uploadImage} />
<DocumentView doc={doc} extensions={EXTENSIONS} />        // elements — server-rendered
renderDocumentHtml(doc, { extensions: EXTENSIONS })      // the same bytes as a string
documentText(doc, { extensions: EXTENSIONS })            // plain text, for a teaser or search

import { DocumentHtml } from "zeb/ui/editor-html";
<DocumentHtml html={row.body_html} />                    // stored HTML, sanitized again — see "Stored HTML"
```

The basics of the editor itself (slash menu, Markdown shortcuts, image
upload) are in `help("web/ui")`; the engine is `zeb/prosemirror`
(`help("web/libraries")`).

---

## Shipped extensions

Each is a factory in its own file; call it once and keep the result.

| file | factory | what it adds |
|---|---|---|
| `zeb/ui/editor-callout` | `calloutExtension()` | a box with an icon and a tone (`note`, `info`, `success`, `warning`, `danger`) around blocks |
| `zeb/ui/editor-table` | `tableExtension()` | a table with a header row; Tab / Shift-Tab / Enter move between cells; the panel adds and deletes rows and columns |
| `zeb/ui/editor-figure` | `figureExtension()` | an image with a caption (typed in place) and a credit; starts from `uploadImage` when the editor has one |
| `zeb/ui/editor-embed` | `embedExtension({ name, label, route })` | a card for a record that lives elsewhere: stored as `key` + `snapshot` |
| `zeb/ui/editor-mention` | `mentionExtension({ name, trigger, route, render })` | a trigger character opens a picker; the choice becomes an inline link with its snapshot |
| `zeb/ui/editor-reference` | `referenceExtension({ name, variant, route, render })` | `/` + its label searches the project's documents; the choice becomes a link (or a small card) with its snapshot |
| `zeb/ui/editor-citation` | `citationExtension({ name, route, heading })` | cite a work inline; the page numbers citations `[1]`, `[2]` and lists them under References |
| `zeb/ui/editor-potoru` | `potoruExtension({ name, route, libraries })` | a Potoru story (`.poto`) played by PotoPlayer, with a live preview and its library list in the editor |

`embed`, `mention`, `reference` and `citation` take `name` so a project can
have several. Instead of `route` they accept `search: async (query) => items`
for a list the page already holds.

---

## Mentions and references: the project decides

Zebflow ships the **mechanism** — a trigger or slash item, a picker, a node
that keeps `{ id | key, label, href, snapshot }`, and its rendering. It
knows nothing about people, organizations, articles or any other collection.
Each project says what can be mentioned or linked, where the suggestions
come from, and how the result looks:

```tsx
// shared/editor/extensions/index.tsx — two mention kinds, a document link, a card
mentionExtension({ name: "person", trigger: "@", route: "/api/people/search" }),
mentionExtension({
  name: "org", trigger: "+", route: "/api/orgs/search",
  render: (node, r) => r.h("a", { class: "font-semibold text-primary", "data-org": node.attrs.id, href: node.attrs.href }, node.attrs.label),
}),
mentionExtension({ name: "topic", trigger: null, route: "/api/topics/search" }),   // slash menu only
referenceExtension({ name: "article", label: "Link to article", route: "/api/articles/search" }),
referenceExtension({ name: "page_card", variant: "card", route: "/api/pages/search" }),
```

- **Each kind is its own extension**, with its own `name` (the node type
  stored in the JSON), its own route and its own `trigger`.
- A `trigger` is **one character, unique in the list**: two kinds on `@`
  would leave the second unreachable, so the list is refused
  (`editor extensions "person" and "org" share the trigger "@"`).
  `trigger: null` puts a kind in the slash menu only. A reference has no
  trigger unless given one; `/` + its `label` opens its picker.
- `render(node, r)` replaces the default look (a chip for a mention, an
  underlined link or a card for a reference). It is used by the editor and
  every renderer alike and passes the same allowlist.
- A **reference** stores `{ key: id, label, href, snapshot }`, like an
  embed. `variant: "link"` (default) is inline; `"card"` is a small block
  showing `snapshot.description` and `snapshot.meta`. A pipeline that
  renames or moves a document may refresh every reference to it (below).

---

## Where a project keeps them

Extensions are ordinary project files under `shared/editor/extensions/`.
One `index.tsx` builds the list once, outside any component, and every page
that edits or shows documents imports it:

```tsx
// shared/editor/extensions/index.tsx
import { calloutExtension } from "zeb/ui/editor-callout";
import { tableExtension } from "zeb/ui/editor-table";
import { figureExtension } from "zeb/ui/editor-figure";
import { embedExtension } from "zeb/ui/editor-embed";
import { mentionExtension } from "zeb/ui/editor-mention";
import { citationExtension } from "zeb/ui/editor-citation";
import { note } from "@/shared/editor/extensions/note";

export const EXTENSIONS = [
  calloutExtension(),
  tableExtension(),
  figureExtension(),
  embedExtension({ name: "product", label: "Product", route: "/api/products/search" }),
  mentionExtension({ name: "person", route: "/api/people/search" }),
  citationExtension({ route: "/api/works/search" }),
  note,
];
```

The editor reads the list when it mounts; a page that renders documents
must pass the **same** list, or the nodes it lacks show as unknown (below).

---

## Picker routes: `{ id, label, href, snapshot }`

`mention`, `reference`, `citation`, `embed` and `potoru` ask a project route while the author
types. The contract is one request and one answer:

- `GET <route>?q=<what was typed>` — same origin, with the page's cookies.
- The answer is a **JSON array** of `{ id, label, href, snapshot }`.
  `id` is the stable key; `label` is shown and stored; `href` is where the
  rendered link goes; `snapshot` is whatever the page needs to render the
  node later without asking again.
- Anything else — a non-2xx status, an object — is shown in the picker as
  an error, not as an empty list.

A route is a pipeline like any other:

```
| trigger.webhook --route /api/people/search --method GET
| sekejap.query.run --param "1=%{{ input.webhook.query.q }}%" -- "SELECT _key AS id, name AS label, page AS href FROM people WHERE name ILIKE $1 LIMIT 8"
| web.response.send --body "{{ input.query.rows }}"
```

For citations the route decides what a query means — a DOI to resolve, a
title in the project's own library — and answers a snapshot
`{ authors, year, title, container, doi, url }`. Zebflow never calls an
outside service for it.

---

## Snapshots, and refreshing them

An embed stores `{ key, snapshot: { title, description, image, href, meta } }`,
a reference `{ key, label, href, snapshot }`, a mention `{ id, label, href, snapshot }`
and a story block `{ key, src, mode, still, time, libraries, snapshot: { title, poster } }`. Rendering reads only
the stored node: a page never fetches to draw one, and a deleted record
leaves a readable card rather than a broken page.

When a record changes, the pipeline that owns it may rewrite the snapshot
inside the stored documents — the JSON is plain data. Walk the document and
replace the snapshot of each node of that type and key:

```
| trigger.function
| javascript.script.run -- "const walk = (n) => n.type === input.type && n.attrs.key === input.key ? { ...n, attrs: { ...n.attrs, snapshot: input.snapshot } } : n.content ? { ...n, content: n.content.map(walk) } : n; return { doc: walk(input.doc) }"
```

---

## Story blocks (Potoru)

`potoruExtension()` adds a **Potoru story** as a block. `/potoru` inserts
one; its panel takes the `.poto` address (a project file such as
`/_files/stories/intro.poto`, or any same-origin or CORS URL), the mode
(`play`, `slide`, `interactive`, or the story's own), a still frame with
its time, a title and a poster image. With `route`, `/potoru` opens a
picker instead: the route answers `{ id, label, href, snapshot }` where
`href` is the `.poto` address, `snapshot.poster` an optional poster and
`snapshot.libraries` the story's own libraries.

```tsx
potoruExtension({ route: "/api/stories/search" })
potoruExtension({ libraries: ["https://example.com/libs/basic.potolib"] })   // every block gets this library
```

- **Rendering.** The block renders exactly the placeholder `PotoPlayer`'s
  server stub renders — `data-zeb-lib="potoru"`, `data-zeb-wrapper="PotoPlayer"`
  and the same `data-config` — inside a `<figure>` with the title as its
  caption. The page hydrates it into the player (with controls). The poster
  sits inside the placeholder and shows until the player mounts.
- **Editing.** The block is drawn by components (below): the editor shows
  the same placeholder, so the story plays as a live preview while it is
  edited, and under it the block's **library list**.
- **Libraries.** A story may need Potoru libraries (`.potolib`), PotoPlayer's
  `libraries`. The extension's `libraries` are **defaults** for every block
  on the page; the author adds or removes a block's **own** in the list
  under the preview (stored in the block's `libraries` attr). The player
  gets both, merged: the defaults, then the block's own; when a default and
  a block entry have the same file name (`…/basic.potolib`) the block's
  entry wins and the default is left out. An address must be `https://` or
  a path on this site (`/_files/libs/basic.potolib`): a bad default throws
  when the extension is built, the list refuses a bad address, and a bad
  stored one is left out of the page. A story that links no library plays
  without fetching them.
- **Loading.** The extension never imports `zeb/potoru`. RWE loads the
  library on a page only when such a placeholder is on it: a page whose
  server HTML holds one, or whose code can draw one (the editor), gets a
  small watcher that imports the bundle the first time a placeholder
  appears — not before, and never on a page without one. A page that
  imports `zeb/potoru` itself loads it as usual.
- **Enable the library.** A project that lists its RWE libraries must list
  `zeb/potoru` (Settings → Libraries, or the hub); otherwise the
  placeholder stays empty.

---

## Writing an extension

`defineExtension` from `zeb/ui/editor-extension` turns one declaration into
everything the editor and the renderers need:

```tsx
// shared/editor/extensions/note.tsx
import { defineExtension } from "zeb/ui/editor-extension";

const TONES = {
  info: "my-3 rounded-md border-l-4 border-info bg-info/10 px-3 py-2",
  warning: "my-3 rounded-md border-l-4 border-warning bg-warning/10 px-3 py-2",
};

export const note = defineExtension({
  name: "note",
  node: { group: "block", content: "inline*", attrs: { tone: { default: "info" } } },
  render: (node, r) => r.h("p", { class: TONES[node.attrs.tone] || TONES.info, "data-note": node.attrs.tone }, r.content),
  parse: [{ tag: "p[data-note]", getAttrs: (dom) => ({ tone: dom.getAttribute("data-note") }) }],
  insert: [{ label: "Note", hint: "A short aside", keys: "note aside", run: (api) => api.insert("note") }],
  fields: [{ name: "tone", label: "Tone", type: "select", options: ["info", "warning"] }],
  inputRules: (schema, pm) => [pm.textblockTypeInputRule(/^!!\s$/, schema.nodes.note)],
});
```

Typing `/note` or `!! ` at the start of a line makes a note; with the caret
inside, a panel sets its tone; `<DocumentView>` renders
`<p data-note="info" class="…">…</p>` on the server.

A node can also be drawn by Zeb React **components** instead of `render`
(next section).

What a declaration may hold:

| key | meaning |
|---|---|
| `name` | unique, `a-z0-9_`; also the node's type when `node` is given |
| `node` / `mark` | the ProseMirror spec — `group`, `content`, `inline`, `atom`, `attrs`, … — without `toDOM`/`parseDOM` |
| `nodes` / `marks` | several types: `{ type: { spec, render, parse, text } }` (the table uses this) |
| `render(node, r)` | the one rendering, used by the editor, `DocumentView` and `renderDocumentHtml` |
| `parse` | ProseMirror `parseDOM` rules, for pasted HTML |
| `text(node)` | the plain-text form of an atom (`documentText`) |
| `footer(r)` | rendered once after the document — the citation list uses it |
| `insert` | slash-menu items: `{ label, hint, keys, run(api) }` |
| `trigger`, `picker` | a character that opens the picker, and `{ search(query), toAttrs(item), type? }` |
| `fields`, `actions` | the panel for a node under the caret: attrs to edit (`snapshot.title` paths work; `type` is text, `url`, `number`, `checkbox` or `select` with `options`) and buttons `{ label, run(editor, node) }` |
| `commands`, `keymap`, `inputRules`, `plugins` | `(schema, pm) =>` engine pieces; `pm` is ProseMirror (`help("web/libraries")`) |
| `component` | a Zeb React component that draws the node on the page — and in the editor, unless `editComponent` is given. Instead of `render` |
| `editComponent` | a Zeb React component that draws the node in the editor only (a live node view) |
| `options` | the extension's configuration, handed to both components as `options` |

`api` in an insert item: `insert(type, attrs)`, `wrap(type, attrs)`,
`exec(command, …args)`, `pick()` (open this extension's picker with a search
box), `upload()` (the editor's `uploadImage`, as a promise of
`{ src, ref, alt }`), `canUpload`, `editor`.

### Rendering: `r`

- `r.h(tag, attrs, ...children)` builds an element. Attributes are written
  as `class`, `href`, `data-*`, `aria-*`.
- `r.content` is the node's children. Put it **alone** inside its element —
  the editor turns it into the editable hole.
- `r.edit` is `true` while the editor draws the node; use it for hints that
  never reach a page (the figure's "no image yet").
- `r.collect(key, id, item)` numbers `id` by first appearance and keeps
  `item`; `r.collected(key)` lists them for `footer`. In the editor it
  answers `null`.

Write class strings as literals in the extension file, so the page's
Tailwind scan sees them. `render` builds with `r.h`, not JSX: the engine
calls it for the editor's DOM. Components (next) are ordinary Zeb React and
may use JSX.

---

## Components: drawing a node with Zeb React

An extension may take components instead of `render` — for a node whose
editing needs real controls, or whose look is easier to write as a
component. Configure it like any extension, with plain `options` beside the
components:

```tsx
// shared/editor/extensions/status.tsx
import { defineExtension } from "zeb/ui/editor-extension";

const TONES = { draft: "bg-muted text-muted-foreground", review: "bg-warning/20 text-warning", done: "bg-success/20 text-success" };

// The page: printed by the server, hydrated with the page, the same bytes from renderDocumentHtml.
function StatusBadge({ attrs, options }) {
  return <span data-status={attrs.state} className={`rounded-full px-2 py-0.5 text-xs ${TONES[attrs.state] || TONES.draft}`}>{options.labels[attrs.state] || attrs.state}</span>;
}

// The editor: the badge and a select that changes the node.
function StatusEditor(props) {
  const { attrs, options, update, readOnly } = props;
  return <span className="inline-flex items-center gap-1">
    <StatusBadge {...props} />
    {readOnly ? null : <select value={attrs.state} onChange={(e) => update({ state: e.target.value })}>
      {Object.keys(options.labels).map((s) => <option key={s} value={s}>{options.labels[s]}</option>)}
    </select>}
  </span>;
}

export function statusExtension({ labels = { draft: "Draft", review: "In review", done: "Done" } } = {}) {
  return defineExtension({
    name: "status",
    node: { group: "inline", inline: true, atom: true, attrs: { state: { default: "draft" } } },
    options: { labels },
    component: StatusBadge,
    editComponent: StatusEditor,
    text: (node) => labels[node.attrs.state] || node.attrs.state,
    insert: [{ label: "Status", hint: "A status badge", keys: "status", run: (api) => api.insert("status") }],
  });
}
```

**Props.** Both components get the same object:

| prop | on the page | in the editor |
|---|---|---|
| `node` | `{ type, attrs }` | the same |
| `attrs` | the stored attrs, a missing one filled with its spec default | the node's attrs |
| `options` | the extension's `options` | the same |
| `edit` | `false` | `true` |
| `selected` | `false` | `true` while the node is selected |
| `readOnly` | `true` | the editor's `readOnly` |
| `update(patch)` | does nothing | merges `patch` into the node's attrs (one undo step) |

**Rules.**

- A node drawn by a component is an **atom**: attrs only, no `content`
  (text the author types inside a block needs `render` and `r.content`).
  A mark cannot be drawn by a component.
- The page's look comes from **one** place: `render` or `component`, never
  both. `editComponent` may be paired with either — with `render`, the
  page uses the declared static render and the editor the component.
- **Server and browser match.** `<DocumentView>` renders `component` as an
  element, so the page's SSR prints it and the page hydrates it — its
  hooks and handlers run in the browser. `renderDocumentHtml` prints the
  same element with zeb/react's `renderToString`: the same bytes as the
  server page. Avoid `useId` in a page component (its numbering depends on
  where the tree starts).
- **The page's output is sanitized.** `component` renders through the
  document allowlist (Safety, below), like `render`: a `script`, a `style`,
  a string `on…` attribute or a `javascript:` link from stored attrs never
  reaches the page. Function props (`onClick`) and `ref` are kept — they
  come from code, never from a document.
- **`editComponent` is editor chrome.** It runs only in the browser, as the
  node's ProseMirror node view (`contenteditable="false"`), is not
  sanitized, and may use inputs, selects and buttons: ProseMirror leaves
  their events to the component. Clicking elsewhere on it selects the node.
  Call `update` from an event handler. When ProseMirror itself serializes
  the node (copy and paste), it writes the attrs as JSON and reads them
  back.

The Potoru story block is built this way: `PotoruBlock` is its `component`
(the player's placeholder) and `PotoruBlockEditor` its `editComponent` (the
preview and the library list).

---

## Safety

Every element of every rendering — `render` and an extension's `component`
alike — passes one allowlist (`zeb/ui/editor-extension`):

- `script`, `style`, `iframe`, `object`, `embed`, `svg`, `form` and similar
  are dropped with their content; any other unlisted tag becomes a `span`.
- Attributes are allowlisted: `class`, `id`, `href`, `src`, `alt`, `title`,
  sizes, `data-*`, `aria-*` and a few more. `on…` handlers, `style` and
  `srcset` are dropped.
- `href`, `src` and `cite` keep only `http(s):`, `mailto:`, `tel:`, `/`,
  `#` and `.` addresses; anything else becomes `#`.
- Text is always escaped. A stored document can describe anything; it
  cannot make the page run anything.

The JSON is the source of truth. HTML a browser sent may be stored beside
it, but is only ever shown through `<DocumentHtml>`, which applies this
allowlist again — see "Stored HTML".

---

## Stored HTML

A project may keep the rendered HTML next to the JSON, in a `body_html`
column: a page then shows a document without the extension list, a search
index or a feed reads it, and nothing renders the JSON on every request.
The JSON stays the source of truth — it is what the editor opens.

**Saving.** The editor page renders the HTML when it saves and sends both;
the save pipeline stores both, unchanged, in a table made once:

```sql
CREATE TABLE posts (_key TEXT PRIMARY KEY, body JSON, body_html TEXT)
```

```tsx
import { renderDocumentHtml } from "zeb/ui/editor-render";
import { EXTENSIONS } from "@/shared/editor/extensions/index";

async function save() {
  const body_html = renderDocumentHtml(doc, { extensions: EXTENSIONS });
  await fetch("/posts/save", { method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ key, doc, body_html }) });
}
```

```
| trigger.webhook --route /posts/save --method POST --auth jwt
| sekejap.query.run --write --param "1={{ input.webhook.body.key }}" --param "2={{ input.webhook.body.doc }}" --param "3={{ input.webhook.body.body_html }}" -- "INSERT INTO posts (_key, body, body_html) VALUES ($1, $2, $3)"
| web.response.send --body "{{ { ok: true } }}"
```

**Showing.** The page reads the row and hands the string to
`<DocumentHtml>`:

```
| trigger.webhook --route /posts/:key --method GET
| sekejap.query.run --param "1={{ input.webhook.params.key }}" -- "SELECT body_html FROM posts WHERE _key = $1"
| web.response.send --template pages/post.tsx
```

```tsx
import { DocumentHtml } from "zeb/ui/editor-html";
export default function Post(input) {
  const row = input.query.rows[0];
  return <article><DocumentHtml html={row && row.body_html} /></article>;
}
```

**Why the server need not trust it.** `<DocumentHtml>` never sets the
string as HTML. It parses it and rebuilds every element through the same
allowlist `renderDocumentHtml` uses (see "Safety"), on the server and again
in the browser — the same tree from the same string, so hydration matches.
A row someone tampered with still cannot put a script, a style, an `on…`
handler, an iframe or a `javascript:` link on the page. HTML that
`renderDocumentHtml` wrote shows exactly as `<DocumentView>` shows its
document, byte for byte.

**Blocks.** A block whose library mounts from its markup — the story block's
player — is in the server HTML, so the page loads that library only when
such a block is there, and the player hydrates as it does under
`<DocumentView>`. A block drawn by a component that needs its own event
handlers on the page is markup only in stored HTML; show that document with
`<DocumentView doc>` instead.

This is the one door for document HTML. `dangerouslySetInnerHTML` stays
refused on every page (`RWE_SECURITY_RAW_HTML`).

When the extension list changes, stored HTML still shows what it showed
when it was saved; render it again from the JSON (`renderDocumentHtml` in a
page that saves, or open and save in the editor) to refresh it.

---

## Unknown nodes

A document may hold a node the current list does not know — written by a
richer editor, or by an extension since removed. It is never dropped:

- `<DocumentView>` and `renderDocumentHtml` render it as
  `<div data-unknown-node="type">` (or a `span` inside text) holding its
  content.
- The editor loads it as a placeholder, keeps its text editable where it
  has text, and saves it back exactly as it was.

---

## Checking an editor page

- `cargo test --test rwe zeb_ui_editor` — server rendering, sanitization,
  numbering, unknown nodes, mention kinds, references, the story block's
  placeholder and when its library loads.
- `node --test tests/rwe/runtime/editor.test.mjs` — the engine: schema
  composition, JSON round trip, table commands, component props and output
  sanitizing, story libraries.
- `node --test tests/rwe/runtime/editor-html.test.mjs` — `<DocumentHtml>`:
  stored HTML shown byte for byte, a tampered row sanitized, the same tree
  every time.
- `cd tests/e2e && npx playwright test editor-extensions editor-components editor-potoru editor-html` —
  type, insert a figure, mention two kinds and link a document through
  routes, save the JSON, render the page; a component-drawn node edited
  through its own control and hydrated on the page; insert a story, add a
  library, preview it, and play it on the rendered page with both
  libraries; save a document's HTML beside its JSON in a table and show it
  with `<DocumentHtml>`, the story's player hydrating and a tampered row
  running nothing.
