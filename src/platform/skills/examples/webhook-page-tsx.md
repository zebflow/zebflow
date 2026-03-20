# Webhook → TSX Page

## What this builds

A server-rendered HTML page triggered by HTTP GET. The query result flows directly into the TSX template as `input`. Standard pattern for every data-driven page in Zebflow.

---

## Core Pattern

```
trigger.webhook → (optional query node) → web.render
```

The upstream node's entire output becomes `input` inside the TSX template. `input.rows` for pg.query results, `input.data` or whatever shape the script returns.

---

## Pipelines

### Simple page — static data via script

```
| trigger.webhook --path /hello --method GET
| script -- "return { message: 'Hello World', ts: Date.now() }"
| web.render --template-path pages/hello.tsx --template-id pages/hello.tsx --route /hello
```

### Page with PostgreSQL list

```
| trigger.webhook --path /programmes --method GET
| pg.query --credential my-pg \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true ORDER BY code"
| web.render --template-path pages/programmes.tsx --template-id pages/programmes.tsx --route /programmes
```

In `pages/programmes.tsx` — `input.rows` is the array of DB rows:

```tsx
const rows = input?.rows ?? [];
return (
  <ul>
    {rows.map(p => <li key={p.unit_id}>{p.title} ({p.code})</li>)}
  </ul>
);
```

### Detail page with path param — `:unit_id` → `$1`

```
| trigger.webhook --path /programmes/:unit_id --method GET
| pg.query --credential my-pg --params-path params.unit_id \
    -- "SELECT unit_id::text, code, title, description FROM academic.academic_unit WHERE unit_id = $1::uuid AND is_active = true"
| script -- "const r = input.rows?.[0]; if (!r) return { __status: 404, error: 'not found' }; return r"
| web.render --template-path pages/programme-detail.tsx --template-id pages/programme-detail.tsx --route /programmes/:unit_id
```

In `pages/programme-detail.tsx`:

```tsx
if (input?.error) return <p>Not found</p>;
return (
  <div>
    <h1>{input?.title?.id}</h1>
    <p>{input?.code}</p>
  </div>
);
```

### Page with query string filter — `?faculty_id=uuid`

```
| trigger.webhook --path /programmes --method GET
| pg.query --credential my-pg --params-expr "[input.query.faculty_id ?? null]" \
    -- "SELECT unit_id::text, code, title->>'id' as title FROM academic.academic_unit WHERE unit_type = 'programme' AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| web.render --template-path pages/programmes.tsx --template-id pages/programmes.tsx --route /programmes
```

---

## Nodes Used

- `trigger.webhook` — GET endpoint; path params in `input.params.<name>`, query string in `input.query.<name>`
- `pg.query` — fetch data; `--params-path params.unit_id` binds `:unit_id` as `$1`
- `script` — 404 guard, data transform; `__status` sets HTTP response code
- `web.render` — TSX template; upstream output = `input` in template

---

## Templates Needed

Use `template_create kind=page name=<page-name>` then `template_write` to fill content.
Access upstream data via `input` — it's whatever the previous node returned.
