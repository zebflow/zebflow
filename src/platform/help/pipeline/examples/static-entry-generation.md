# Static Entry Generation

## What this builds

One persisted static page for one entry.

The pipeline does not answer the current HTTP request. Instead it:
- collects one collection/entry payload
- renders a TSX page through RWE
- writes the resulting HTML into `files/public/...`

That makes it the right primitive for any regeneration flow where one content mutation can fan out into many related pages later.

---

## Template

Use a normal TSX page template. A concrete example lives at:

- `docs/unused/conventions/templates/pages/static-entry-page.tsx`
- `docs/unused/conventions/pipelines/static_entry_generation.zf.json`

In a real project, that template would live under:

- `repo/pipelines/pages/static-entry-page.tsx`

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

---

## Pipeline

Minimal version as a callable function pipeline:

```zf
| trigger.function --params '{"entry_slug": {"type": "string", "description": "Slug of the entry to generate"}}'
| script -- "
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
| web.static.generate \
    --template pages/static-entry-page.tsx \
    --scope public \
    --output-path "collections/{{ $input.collection.slug }}/{{ $input.entry.slug }}/index.html" \
    --route "/collections/{{ $input.collection.slug }}/{{ $input.entry.slug }}" \
    --on-conflict overwrite
```

Generated file:

- `files/public/collections/field-notes/city-garden/index.html`

Served URL:

- `/files/{owner}/{project}/public/collections/field-notes/city-garden/index.html`

---

## Database-backed version

This is a more realistic content-backed version:

```zf
| trigger.function --params '{"entry_id": {"type": "string", "description": "Entry UUID"}}'
| pg.query --credential content-db --params-expr "{{ [$input.entry_id] }}" -- "
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
| script -- "
const row = input.rows?.[0];
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
| web.static.generate \
    --template pages/static-entry-page.tsx \
    --scope public \
    --output-path "collections/{{ $input.collection.slug }}/{{ $input.entry.slug }}/index.html"
```

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
3. call `web.static.generate` once per artifact

That is much easier to debug than hiding traversal and parallelism inside one giant node.

---

## Serving behavior

After generation, the file is already serveable directly from project storage.

If the output path is:

- `files/public/collections/field-notes/city-garden/index.html`

then the office can serve it immediately at:

- `/files/{owner}/{project}/public/collections/field-notes/city-garden/index.html`

So:
- generation pipeline: `trigger.function` is enough
- serving generated artifact: no webhook is required

Only add a webhook or ingress rewrite if you want a prettier public route like:

- `/collections/field-notes/city-garden`

instead of the native `/files/...` address.

---

## Separate nginx publish surface

For production static publishing, prefer a separate static host or ingress instead of serving generated HTML from the same origin as Zebflow admin.

Example artifact root inside the project data volume:

- `users/superadmin/default/files/private/static/musiklib`

Example nginx server:

```nginx
server {
  listen 80;
  server_name musiklib.org;

  root /data/users/superadmin/default/files/private/static/musiklib;
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

- `a/index.html` -> `https://musiklib.org/a/`
- `a/aurora/index.html` -> `https://musiklib.org/a/aurora/`
- `a/aurora/songs/runaway/lyrics/index.html` -> `https://musiklib.org/a/aurora/songs/runaway/lyrics/`

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
