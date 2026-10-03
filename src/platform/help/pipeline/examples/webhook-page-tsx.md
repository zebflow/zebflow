# Webhook → TSX Page

## What this builds

A server-rendered HTML page triggered by HTTP GET. The query result flows directly into the TSX template as `input`. Standard pattern for every data-driven page in Zebflow.

---

## Core Pattern

```
trigger.webhook → (optional query node) → web.response.send --template pages/foo.tsx
```

The upstream node's entire output becomes `input` inside the TSX template. `input.query.rows` for pg.query.run results; for a `javascript.script.run` (TypeScript: `typescript.script.run`) node, the template reads `input.script` — whatever shape the script returned, nested under that key, with the rest of the payload still there too.

---

## Pipelines

### Simple page — static data via script

```
| trigger.webhook --route /hello --method GET
| javascript.script.run -- "return { message: 'Hello World', ts: Date.now() }"
| web.response.send --template pages/hello.tsx
```

### Page with PostgreSQL list

```
| trigger.webhook --route /programmes --method GET
| pg.query.run --credential my-pg \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true ORDER BY code"
| web.response.send --template pages/programmes.tsx
```

In `pages/programmes.tsx` — `input.query.rows` is the array of DB rows:

```tsx
const rows = input?.query?.rows ?? [];
return (
  <ul>
    {rows.map(p => <li key={p.unit_id}>{p.title} ({p.code})</li>)}
  </ul>
);
```

### Detail page with path param — `:unit_id` → `$1`, 404 when missing

A script cannot set the response status, so the not-found branch is a real
graph branch, not a flag on the same `web.response.send` that serves the found
case:

```
[find]  trigger.webhook --route /programmes/:unit_id --method GET
[query] pg.query.run --credential my-pg --param "1={{ input.webhook.params.unit_id }}" -- "SELECT unit_id::text, code, title, description FROM academic.academic_unit WHERE unit_id = $1::uuid AND is_active = true"
[found] logic.if --expr "input.query.rows && input.query.rows.length > 0"
[ok]    web.response.send --template pages/programme-detail.tsx
[gone]  web.response.send --status 404 --template pages/not-found.tsx
[find] -> [query]
[query] -> [found]
[found]:true -> [ok]
[found]:false -> [gone]
```

`[ok]` receives `{ query: { rows } }` as `input` in `pages/programme-detail.tsx`:

```tsx
export default function Page(input) {
  const row = input?.query?.rows?.[0];
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
| trigger.webhook --route /programmes --method GET
| pg.query.run --credential my-pg --param "1={{ input.webhook.query.faculty_id ?? null }}" \
    -- "SELECT unit_id::text, code, title->>'id' as title FROM academic.academic_unit WHERE unit_type = 'programme' AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| web.response.send --template pages/programmes.tsx
```

---

## Nodes Used

- `trigger.webhook` — GET endpoint; path params in `input.webhook.params.<name>`, query string in `input.webhook.query.<name>` right after the trigger (`$trigger.params`, `$trigger.query` anywhere later)
- `pg.query.run --credential <id>` — fetch data; `--param "1={{ input.webhook.params.unit_id }}"` binds `:unit_id` as `$1`
- `logic.if --expr "input.query.rows.length > 0"` — branch on `true`/`false` pins; the only way to answer 404 conditionally, since a script cannot set the status
- `javascript.script.run` (TypeScript: `typescript.script.run`) — static payloads, data transform; its answer sits under `script` in the payload (`input.script.<field>`), the rest is kept
- `web.response.send` — renders TSX template; upstream output = `input` in template; supports `--status` and `--header` (`Set-Cookie`, `Location`, …)

---

## Templates Needed

Use `file_create kind=page name=<page-name>` then `file_write` to fill content.
Access upstream data via `input` (function parameter) — it's whatever the previous node returned.
`ctx` is the same object available as `globalThis.ctx` in both SSR and browser.
