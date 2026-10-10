# Static Entry Generation

## What this builds

One persisted static page for one entry.

The pipeline does not answer the current HTTP request. Instead it:
- collects one collection/entry payload
- renders a TSX page through RWE
- writes the resulting HTML into Zebflow FS

That makes it the right primitive for any regeneration flow where one content mutation can fan out into many related pages later.

---

## Template

Use a normal TSX page template. A project's source root is the repository
root (unless `zebflow.yaml` sets `spec.layout.source`), so the template lives
at:

- `pages/static-entry-page.tsx`

The template receives a single input payload like:

```json
{
  "collection": {
    "name": "Field Notes",
    "slug": "field-notes"
  },
  "entry": {
    "title": "City Garden",
    "slug": "city-garden",
    "summary": "A compact example of one generated static content page.",
    "body": "A small archive can still feel deliberate.\n\nThis example shows how one payload becomes one persisted HTML artifact."
  },
  "related_labels": [
    { "slug": "example", "label": "Example" },
    { "slug": "static", "label": "Static" }
  ],
  "generated_at": "2026-04-11T05:02:00Z"
}
```

When a `javascript.script.run` node builds this shape (as in the pipelines below), it
is the script's own answer, so anything downstream reads it one level deeper —
`input.script.collection`, `input.script.entry`, `input.script.related_labels`,
`input.script.generated_at` — since the script node now keeps the rest of the
payload and adds its own answer under `script` instead of replacing it.

**What the page publishes.** The page renders from the payload and embeds it
for hydration, so the generated file carries every key the payload holds —
shape it first. The one exception is the request: a trigger's `headers`,
`cookies` and `auth` (`input.webhook.headers`, `input.webhook.auth`, …) are
never part of a generated page. A value the page should show — a caller's
public profile, say — is copied into a key of its own by an upstream node.

---

## Pipeline

Minimal version as a callable function pipeline:

```zf
| trigger.function --description "Generate the static page for one entry" --parameter "entry_slug:string!" "Slug of the entry to generate"
| javascript.script.run -- "
const collection = {
  name: 'Field Notes',
  slug: 'field-notes'
};
const entry = {
  title: 'City Garden',
  slug: 'city-garden',
  summary: 'A compact example of one generated static content page.',
  body: `A small archive can still feel deliberate.

This example shows how one payload becomes one persisted HTML artifact.`
};
return {
  collection,
  entry,
  related_labels: [
    { slug: 'example', label: 'Example' },
    { slug: 'static', label: 'Static' }
  ],
  generated_at: new Date().toISOString()
};
"
| web.site.generate \
    --template pages/static-entry-page.tsx \
    --path "collections/{{ input.script.collection.slug }}/{{ input.script.entry.slug }}/index.html" \
    --route "/collections/{{ input.script.collection.slug }}/{{ input.script.entry.slug }}" \
    --on-conflict overwrite
```

Generated file:

- `site/collections/field-notes/city-garden/index.html`

Served URL, once the owner serves `site/` as a site on `https://www.example.com`:

- `https://www.example.com/collections/field-notes/city-garden/`

---

## Database-backed version

This is a more realistic content-backed version:

```zf
| trigger.function --description "Generate the static page for one content entry" --parameter "entry_id:string!" "Entry UUID"
| postgres.query.run --credential content-db --param "1={{ input.function.entry_id }}" -- "
SELECT
  e.entry_id::text AS entry_id,
  e.slug AS entry_slug,
  e.title AS entry_title,
  e.summary AS entry_summary,
  e.body AS entry_body,
  c.collection_id::text AS collection_id,
  c.slug AS collection_slug,
  c.name AS collection_name
FROM content.entry e
JOIN content.collection c ON c.collection_id = e.collection_id
WHERE e.entry_id = $1::uuid
"
| javascript.script.run -- "
const row = input.query.rows?.[0];
if (!row) throw new Error('entry not found');
return {
  collection: {
    id: row.collection_id,
    slug: row.collection_slug,
    name: row.collection_name
  },
  entry: {
    id: row.entry_id,
    slug: row.entry_slug,
    title: row.entry_title,
    summary: row.entry_summary,
    body: row.entry_body
  },
  related_labels: [],
  generated_at: new Date().toISOString()
};
"
| web.site.generate \
    --template pages/static-entry-page.tsx \
    --path "collections/{{ input.script.collection.slug }}/{{ input.script.entry.slug }}/index.html"
```

---

## A whole docs site — `--from`

The same node builds a documentation site from a Markdown folder under
`docs/`: one page per `.md`, a sidebar from the tree (each folder's
`_meta.yaml` gives its `title`, `order`, `collapsed` and `nav`), components
from fenced directives, math, a search index, and the site's machine-readable
surface.

```zf
| trigger.schedule --cron "0 3 * * *"
| web.site.generate --from handbook --folder handbook-site --name "Example Handbook"
```

One order — a folder's `nav` (the names listed come first, in that order),
then `order` (a folder's in its `_meta.yaml`, a page's in its front matter;
unordered items follow), then title — drives the sidebar, prev/next and the
search index; the root `index.md` is first, a folder's `index.md` before its
pages. A page whose Markdown was removed or renamed leaves the site on the
next build: its file, its manifest entry, its sitemap and index lines.

`numbered: true` in the **root** `_meta.yaml` makes the site a book: the
sidebar carries `1`, `1.1`, `1.4.1`, as deep as the folders go. The number is
counted from that same single order, so it can never disagree with prev/next
or the search index, and the root `index.md` carries none — it is the front
door a reader lands on, not the first chapter. The switch is read at the root
only: a site is a book or it is not.

`--template` is the page template under the source root (default
`docs.template.tsx`); when the file does not exist it is written from a
scaffold, and edited from then on. `--path` and `--from` choose the mode —
give one, never both.

### What a docs build writes besides the pages

Every build, with no flag and nothing authored by hand
(`contracts/discoverability.md` §1 and §3):

| File | What it holds |
|---|---|
| `sitemap.xml` | every page that is not `noindex`, each with a `lastmod` from the date its Markdown was last written |
| `robots.txt` | the sitemap's address, and a `Disallow` per `noindex` page |
| `llms.txt` | the site in one page: title, summary, and every page as a link with its description |
| `llms-full.txt` | the same order, with every page's Markdown |
| `<route>/index.md` | each page's Markdown twin, beside its HTML |
| `search/…` | the search index, in chunks (below) |

Addresses in those files are absolute once the folder is served on an
address, and the page's own route before then; the next build rewrites them.
Each page's head also carries `canonical`, the `og:*` and `twitter:*` set
(defaults from its own title and description), a `TechArticle` and
`BreadcrumbList` in JSON-LD, and a `rel="alternate" type="text/markdown"`
link to its twin.

### Components in Markdown, without MDX

A page is handed to its template as blocks, not as one string. Plain Markdown
is one `markdown` block; a fenced directive is a block of its own, and the
template renders each one with a component of its own. The set is closed — a
directive cannot name anything that is not in the list, and nothing in a `.md`
file becomes code the page runs:

| Written | Block | Rendered by |
|---|---|---|
| `:::note` `:::tip` `:::info` `:::warning` `:::caution` `:::danger`, with an optional title on the opening line | `admonition` | `Admonition` |
| `:::tab Label`, one per pane; consecutive panes are one tab set, and a `::::tabs` wrapper is optional | `tabs` | `TabbedPanes` |
| `:::cards` holding an ordinary Markdown list, `- [Title](./page.md) — description` | `cards` | `CardGrid` |
| `:::code-group` holding one fenced block per language, labelled by `title="npm"` or `[npm]` | `code_group` | `CodeGroup` |
| a fenced code block at the left margin | `code` | `CodeBlock` |

Restyle every admonition on the site by editing one component in the
template. Directives nest by colon count — the outer one takes four colons,
the one it holds three. A directive inside a fenced code block stays text, so
a page can document the syntax. An unknown name (`:::carousel`) renders its
content plainly rather than nothing, and never as markup.

### Code blocks

A fenced block at the left margin is a block of its own, and its info string
says what the page draws around it: `ts title="server.ts" {3,7-9}`.

| Written | What it does |
|---|---|
| `title="server.ts"`, or `[server.ts]` | a filename header above the code |
| `{3,7-9}` | those lines tinted, numbered from one |
| the language `diff` | `+` and `-` drawn in a column of their own, the lines tinted |

Every block has a copy button and a wrap toggle. Wrapping is off by default,
because a wrapped command line is a misread command line, and the toggle is
per block rather than per site.

The copy button copies code, never markup: a `+` is a mark on the line and not
part of its text, so copying a diff gives the file after the change — its
removed lines and its `@@` headers are shown and not copied.

A fence **indented inside a list item** belongs to that list and stays in the
Markdown that holds it, where the Markdown renderer draws it. Pulling it out
would end the list at the fence and start a new one after it.

### Math, without a math library

`$…$` and `$$…$$` are rendered while the site is built, into **MathML** — the
notation every current browser draws itself. The folder stays a folder of
pages: no JavaScript library to download, no stylesheet, and no font
directory. An equation is text a reader can select and a screen reader can
read aloud.

```md
The probability of $a$ is

$$P(a) = |\langle a|\psi\rangle|^2$$
```

What is understood is the subset a documentation page reaches for: fractions,
roots, sub- and superscripts, sums and integrals with their limits, matrices
(`\begin{pmatrix}`), `\left…\right` delimiters, accents, the Greek alphabet
and the usual operators and function names. Nothing is guessed at — a command
outside that set leaves the author's own `$…$` source on the page, so one
unusual macro never costs a build.

A `$` is only math when it reads like math: the opening one must be followed
by something other than a space and the closing one preceded by the same, so
"it costs $5 and $7" is two prices. `\$` is a dollar sign, and a `$` inside a
code span or a fenced block is left alone, so a page can document the syntax.

The source — not the MathML — is what goes into the Markdown twin,
`llms-full.txt` and the search index, and a heading keeps its slug from its
own text, so an equation in a heading never becomes an unreadable anchor.

### Links are checked before the site is written

Every internal link and `#anchor` in the Markdown is resolved against the
routes and headings the build is about to write. A link to a page that is not
there, an anchor for a heading that was renamed, or a path climbing out of the
docs folder refuses the build and names every one of them at once — nothing is
written. External links (`http`, `https`, `mailto`, `tel`) are never fetched
and never checked, and a link inside a fenced code block is left alone.

### Search without a server

The index is split, so a query fetches the part its own terms live in:

| File | What it holds |
|---|---|
| `search/manifest.json` | the chunk ids — the only names the page may fetch |
| `search/t-<prefix>.json` | the pages each term appears in, with a weight; keyed by the term's first letter, split to two letters once a letter holds more than 400 terms |
| `search/m-<block>.json` | the result card (title, address, section, description) for 64 pages |
| `search/x-<page>.json` | one page's text, for the matched line under a result |

A two-term query over sixty pages costs about 20 KB: the manifest, two term
chunks, one card block, and the text of only the results shown. A thousand
pages cost the same, which is the point of the split.

The page tokenises a query exactly as the build tokenised the Markdown — lower
case, split on everything that is not a letter or a digit, two characters to
thirty-two. A term chooses its chunk **from the manifest's list**; it never
spells a file name, so no query can name a path. The query is capped at 128
characters and 8 terms, prefix matching is bounded inside one already-fetched
chunk, nothing is compiled into a regular expression, and the matched line is
located by index and rendered as text — a page whose Markdown contains markup
cannot write markup into a result.

---

## Why this is the correct first step

This node should stay single-target first.

One run should generate one artifact. That gives:
- clear observability
- clear retries
- easier invalidation logic
- no giant opaque batch node

Later, when one label rename touches thousands of pages, the pipeline should:
1. compute the dirty entry/collection/tag set
2. loop or fan out over that set
3. call `web.site.generate` once per artifact

That is much easier to debug than hiding traversal and parallelism inside one giant node.

---

## Serving behavior

Everything generated is private, like any uploaded file: a folder name never
exposes it, and no node can. The owner exposes the output folder in Studio →
Files:

- **Serve as a site** (`public_execute`) on the addresses written out in its
  `serve` list — each a host the project has in Settings → Addressing. That
  address answers the folder at `/`: `/collections/field-notes/city-garden/`
  reads `site/collections/field-notes/city-garden/index.html`, scripts running.
- **Public read** (`public_read`) instead makes the files readable on the
  project's file host, but never run as a page — a downloaded HTML file, not a
  site.

So:
- generation pipeline: `trigger.function` is enough
- serving generated artifact: no webhook is required once its folder is
  served as a site

Only add a webhook or ingress rewrite if you want a prettier public route like:

- `/collections/field-notes/city-garden`

instead of the native `/files/...` address.

---

## Separate nginx publish surface

For production static publishing, prefer a separate static host or ingress instead of serving generated HTML from the same origin as Zebflow admin.

Example artifact root inside the project data volume:

- `users/superadmin/default/files/static/site-a`

Example nginx server:

```nginx
server {
  listen 80;
  server_name site-a.example;

  root /data/users/superadmin/default/files/static/site-a;
  index index.html;

  location / {
    try_files $uri $uri/ $uri/index.html =404;
  }

  location /_assets/ {
    expires 30d;
    add_header Cache-Control "public, max-age=2592000, immutable";
  }

  location ~* \.html$ {
    expires -1;
    add_header Cache-Control "no-cache";
  }
}
```

That mapping makes these files resolve directly:

- `a/index.html` -> `https://site-a.example/a/`
- `a/demo-author/index.html` -> `https://site-a.example/a/demo-author/`
- `a/demo-author/notes/first-note/index.html` -> `https://site-a.example/a/demo-author/notes/first-note/`

The generator is only writing artifacts. The nginx host is the publishing surface.

---

## Security boundary

Static generation is not the risky part. The real trust boundary is **where the generated HTML executes** and **who can influence the generated bytes**.

Use this rule:

- Hyperguard all project-user-generated or project-user-influenceable content from the platform origin.

That means:

- Platform-authored static pages can be trusted more.
- Project-user-generated content should be strongly guarded and isolated from platform origin.

In practice:

- trusted platform-maintained static pages may be acceptable on the main platform origin
- project-user-authored or project-user-influenceable HTML should not execute on the same origin as project studio, admin pages, API routes, or platform session cookies

If you need public HTML hosting for lower-trust content, prefer a separate publishing origin instead of serving executable HTML on the main Zebflow origin.
