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
