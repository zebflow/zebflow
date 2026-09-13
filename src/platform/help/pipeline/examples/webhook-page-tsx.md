# Webhook → TSX Page

## What this builds

A server-rendered HTML page triggered by HTTP GET. The query result flows directly into the TSX template as `input`. Standard pattern for every data-driven page in Zebflow.

---

## Core Pattern

```
trigger.webhook → (optional query node) → web.response --template pages/foo.tsx
```

The upstream node's entire output becomes `input` inside the TSX template. `input.rows` for pg.query results, `input.data` or whatever shape the script returns.

---

## Pipelines

### Simple page — static data via script

```
| trigger.webhook --path /hello --method GET
| script -- "return { message: 'Hello World', ts: Date.now() }"
| web.response --template pages/hello.tsx
```

### Page with PostgreSQL list

```
| trigger.webhook --path /programmes --method GET
| pg.query --credential my-pg \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true ORDER BY code"
| web.response --template pages/programmes.tsx
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

### Detail page with path param — `:unit_id` → `$1`, 404 when missing

A script cannot set the response status, so the not-found branch is a real
graph branch, not a flag on the same `web.response` that serves the found
case:

```
[find]  trigger.webhook --path /programmes/:unit_id --method GET
[query] pg.query --credential my-pg --params "{{ input.params.unit_id }}" -- "SELECT unit_id::text, code, title, description FROM academic.academic_unit WHERE unit_id = $1::uuid AND is_active = true"
[found] logic.if --expr "input.rows && input.rows.length > 0"
[ok]    web.response --template pages/programme-detail.tsx
[gone]  web.response --status 404 --template pages/not-found.tsx
[find] -> [query]
[query] -> [found]
[found]:true -> [ok]
[found]:false -> [gone]
```

`[ok]` receives `{ rows }` as `input` in `pages/programme-detail.tsx`:

```tsx
export default function Page(input) {
  const row = input?.rows?.[0];
  return (
    <Page>
      <main>
        <h1>{row?.title?.id}</h1>
        <p>{row?.code}</p>
      </main>
    </Page>
  );
}
```

### Page with query string filter — `?faculty_id=uuid`

```
| trigger.webhook --path /programmes --method GET
| pg.query --credential my-pg --params "{{ [input.query.faculty_id ?? null] }}" \
    -- "SELECT unit_id::text, code, title->>'id' as title FROM academic.academic_unit WHERE unit_type = 'programme' AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| web.response --template pages/programmes.tsx
```

---

## Nodes Used

- `trigger.webhook` — GET endpoint; path params in `input.params.<name>`, query string in `input.query.<name>`
- `pg.query --credential <id>` — fetch data; `--params "{{ input.params.unit_id }}"` binds `:unit_id` as `$1`
- `logic.if --expr "input.rows.length > 0"` — branch on `true`/`false` pins; the only way to answer 404 conditionally, since a script cannot set the status
- `script` — static payloads, data transform
- `web.response` — renders TSX template; upstream output = `input` in template; supports `--status`, `--set-cookie`, `--header`

---

## Templates Needed

Use `file_create kind=page name=<page-name>` then `file_write` to fill content.
Access upstream data via `input` (function parameter) — it's whatever the previous node returned.
`ctx` is the same object available as `globalThis.ctx` in both SSR and browser.
