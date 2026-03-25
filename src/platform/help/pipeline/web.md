# Pipeline DSL — Web pages (`n.web.render`)

This doc is about **serving HTML pages** from pipelines: trigger → data nodes → **`n.web.render`** → browser.

See also: **`pipeline-dsl`** (full DSL), **`web-templates`** (how to write `.tsx` pages).

---

## What `n.web.render` does

1. Upstream nodes build a JSON **`input`** for the page.
2. The template file (TSX under `repo/pipelines/`) is compiled and rendered to HTML on the server.
3. If the page is interactive, the platform sends a small client script to hydrate.

---

## Minimal pipeline shape

Use a real `file_rel_path` for the pipeline file (see `help_pipeline`). Example body:

```zf
| trigger.webhook --path /blog --method GET
| pg.query --credential my-db -- "SELECT id, title, published_at FROM posts ORDER BY published_at DESC LIMIT 20"
| n.web.render --template-path pages/blog-home
```

- The **webhook path** is what users open in the browser.
- **`--template-path`** is the page path under `repo/pipelines/` **without** `.tsx`.

---

## Node settings (short)

| Field | Meaning |
|--------|---------|
| `template_path` | Required. Example: `pages/blog-home`. |
| `route` | Often comes from the trigger; set in the node when you need an override. |
| `load_scripts` | Optional list of allowed external script URLs (project allow-list). |

---

## Where templates live

**`repo/pipelines/`** — e.g. `pages/...`, `components/...`, `shared/ui/...`. Imports use **`@/`** from that root.

---

## Page code

The default export receives **`input`**:

```tsx
export default function BlogHome(input: { rows?: any[] }) {
  const posts = input?.rows ?? [];
  return (
    <div>
      <h1>Blog</h1>
      <ul>
        {posts.map((p) => (
          <li key={p.id}><a href={`/blog/${p.id}`}>{p.title}</a></li>
        ))}
      </ul>
    </div>
  );
}
```

Hooks (`useState`, …) are globals; you may add `import { useState } from "zeb"` on pages for editor hints — see **`web-templates`**.

---

## Imports (rules)

Only **`zeb`**, **`zeb/...`** (enabled libraries), and **`@/...`**. Full detail: **`web-templates`** and **ARCHITECTURE** §11.

---

## Example: post detail

```zf
| trigger.webhook --path /blog/:id --method GET
| pg.query --credential main-db -- "SELECT * FROM posts WHERE id = $1"
| n.web.render --template-path pages/blog-post
```

`repo/pipelines/pages/blog-post.tsx` — same `input` pattern as above; use `input.rows?.[0]` for one row.
