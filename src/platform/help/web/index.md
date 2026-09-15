# Web pages (TSX)

A page is a TSX file in the project, rendered to HTML on the server and
hydrated in the browser. A pipeline serves it:

```
| trigger.webhook --path /posts/:slug --method GET
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response --template pages/post.tsx
```

The payload that reaches `web.response` is the page's `input`. Save the file
with `file_write`; the next request renders the new file — there is no build
step. Paths are relative to the project's source root, which is the repository
root unless `zebflow.yaml` sets `spec.layout.source`.

---

## Import rules

**Every file imports what it uses, from `"zeb/react"`.** Entry pages and
component files alike. There is one door; `react`, `preact`, npm, CDN and
relative paths are refused at compile time.

```tsx
import { useState, useEffect, cx } from "zeb/react";
import { Button } from "zeb/ui/button";            // component library, no install
import PostCard from "@/components/post-card";     // your own file: always @/, never ../
import "@/globals.css";                            // the project's theme tokens
```

- A hook used without an import is refused: `'useState' is used but not imported`.
- `@/` is the source root. Relative imports are refused — the compiler would
  resolve one but not carry it into the bundle.
- The compiler inlines every imported file into **one flat bundle per page**.
  Two consequences: a component must import its own names (it never inherits
  the page's), and two files may not export the same name onto one page
  (`RWE_BUNDLE_NAME_COLLISION`). Non-exported top-level names are prefixed
  per file, so they never collide.
- CSS: `import "@/globals.css"` and `import "@/styles/x.css"` work; a directory
  import loads its `index.css`; duplicates are loaded once.

What `zeb/react` exports: `help("web/hooks")`. What is refused and why:
`help("web/custom-scripts")`.

---

## Page file shape

```tsx
// pages/post.tsx
import { useState } from "zeb/react";
import { Button } from "zeb/ui/button";
import "@/globals.css";

export default function Post(input) {
  const post = input.rows?.[0];
  const [liked, setLiked] = useState(false);
  return (
    <main className="mx-auto max-w-2xl p-8">
      <h1 className="text-3xl font-bold">{post?.title}</h1>
      <Button variant="outline" onClick={() => setLiked(!liked)}>{liked ? "Liked" : "Like"}</Button>
    </main>
  );
}

export const page = {
  head: { title: "Post", description: "One post" },
  html: { lang: "en" },
  body: { className: "bg-background text-foreground" },
};

// Optional: config that depends on the data. Merged over `page`.
export function getPage(input) {
  const post = input.rows?.[0];
  return { head: { title: post?.title, og: { title: post?.title, image: post?.cover } } };
}
```

`page.head` fields, each optional:

| Field | Output |
|---|---|
| `title`, `description`, `themeColor`, `robots`, `canonical`, `manifest` | the matching `<title>`, `<meta>` or `<link>` |
| `icons` | `[{ rel, href, type?, sizes? }]` → `<link>` tags (favicons, apple-touch-icon) |
| `links` | `[{ rel, href, type?, sizes?, media?, crossorigin? }]` → `<link>` tags — a stylesheet the page needs |
| `scripts` | `[{ src, defer?, async?, nomodule?, type?, crossorigin?, integrity?, referrerpolicy? }]` → `<script>` tags |
| `og` | `{ title, description, image, url, type, siteName, locale }` → `og:*`. **Defaults**: `og.title` ← `title`, `og.description` ← `description`, `og.url` ← `canonical` or the page's URL, `og.type` ← `website` — set only what differs |
| `twitter` | `{ card, title, description, image, site, creator }` → `twitter:*`; `card` defaults to `summary_large_image` when there is an image |
| `titleSuffix` | appended to `<title>` (`" — RESEARCHSITE"`); set once in the shell's page config; a title equal to the site name takes none |
| `jsonld` | an object or an array of objects → one `<script type="application/ld+json">` each, serialised and escaped by the renderer (`{ "@context": "https://schema.org", "@type": "Person", … }`) |
| `alternates` | `{ en: "/x", id: "/id/x", "x-default": "/x" }` → `<link rel="alternate" hreflang>` per entry |
| `extra` | a raw HTML string appended to `<head>`, unescaped |

**URLs are made absolute for you.** `canonical`, `og.url`, `og.image`,
`twitter.image` and `alternates` written as `/path` come out as
`https://<the host the request came in on>/path`, so the same page is right
in dev and in production and the project never stores its address
(`docs/contracts/addressing.md`, `discoverability.md`).

`html.lang` and `body.className` set the two outer elements. Use
`className`, never `class`.

---

## `input` — data from the pipeline

`input` **is** the payload the previous node produced, with the request
context merged in at the top level (never overwriting a key the pipeline set):

| Key | Value |
|---|---|
| `route` | the path the request arrived on |
| `params` | route parameters (`/posts/:slug` → `{ slug }`) |
| `query` | parsed query string; `search` is the raw `?…` string |
| `headers` | request headers |
| `auth` | the verified token's public claims, when the trigger had `--auth-*` |

So after `sekejap.query`, `input.rows` is the result; after `script -- "return { base: '/x' }"`,
`input.base` is `/x`. There is no `input.state` or `input.request` wrapper.

Server data comes from `input`. Client state is `useState` (local) or
`usePageState("key", default)` (shared across the page's components). See
`help("web/hooks")`.

---

## Hydration

Every page is rendered on the server and hydrated in the browser. To defer a
subtree, put `hydrate` on its element:

```tsx
<section hydrate="onview">…</section>      {/* hydrates when scrolled into view */}
<section hydrate="oninteract">…</section>  {/* hydrates on first click or focus */}
<section hydrate="off">…</section>         {/* server HTML only, never hydrates */}
```

Navigation between pages of the project is client-side through `<Link>` and
`useRouter()`: the next page is fetched and swapped in without a full reload.

---

## Verify a page

A component that throws during server render is replaced by
`<!-- RWE component error: … -->` **and the response is still 200**. After
writing a page or a component it imports:

1. `file_write` (a save evicts every compiled page that inlined the file; if
   something still looks old, `POST /api/projects/{o}/{p}/rwe/cache/clear`).
2. Fetch the page and search the body for `RWE component error`.
3. Open it in a browser; a console error means hydration failed even though
   the server HTML looked right.

---

## MCP workflow

```
file_create   kind=page  name=post  parent_rel_path=pages    → pages/post.tsx scaffold
file_write    rel_path=pages/post.tsx  content="..."
pipeline_register  … | web.response --template pages/post.tsx
pipeline_activate
```

Reusable TypeScript lives in `scripts/*.ts` (`file_create kind=script`) and is
imported with `@/scripts/<name>`; see `help("web/custom-scripts")`.

---

## Further reading

- `help("web/hooks")` — everything `zeb/react` exports; `usePageState`, `useRouter`, `Link`, `cx`
- `help("web/ui")` — zeb/ui: the shadcn component set, the editor, clone-to-own
- `help("web/tailwind")` — the Tailwind subset, theme tokens, `tw-variants`
- `help("web/libraries")` — `zeb/*` runtime libraries: d3, deckgl, codemirror, markdown, pdf, prosemirror, threejs, graphui, livegeo
- `help("web/custom-scripts")` — `.ts` modules and the compiler's refusals
- `help("web/design-system")` — the platform's own theme and component conventions
