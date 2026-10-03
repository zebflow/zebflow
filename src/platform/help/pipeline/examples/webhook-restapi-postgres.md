# REST API + PostgreSQL

## What this builds

JSON REST API endpoints backed by PostgreSQL. Covers list, detail (path param), create, update, delete. Uses `--param` (repeated `key=value`, the key the bind position) for safe parameterized queries — no string interpolation, no SQL injection risk.

---

## Key Concept: Parameter Binding

Path params (`:id`), query strings (`?status=x`), and body fields all land in the webhook input payload. Bind them safely to SQL `$1`, `$2`, ... via:

- `--param "1={{ input.params.id }}"` — one `--param` per placeholder, the key is its position → `$1`
- `--param "1={{ input.name }}" --param "2={{ input.email }}"` — repeat the flag, one key each → `$1, $2, ...`

**Webhook input shape:** user-submitted data lives under `input.body`; request
context sits at the root. Nothing is merged to the root — a JSON body field
`name` is `input.body.name`, never `input.name`.

| Location | Access | Example |
|----------|--------|---------|
| Path param `:id` | `input.params.id` | `--path /api/users/:id` |
| Query string `?status=x` | `input.query.status` | `?status=active` |
| GET request | `input.body` is `null` | path params and query are still at `input.params` / `input.query` |
| POST JSON body field | `input.body.name` | `application/json` — the parsed object sits at `input.body` |
| POST form field | `input.body.name` | `application/x-www-form-urlencoded`, percent-decoded |
| POST multipart text field | `input.body.name` | `multipart/form-data` text fields |
| POST multipart file | `input.files.avatar` | a FileRef object: `{ __zf_type: "file_ref", ref, filename, mime, kind, size, sha256, ... }` |
| Raw / non-object body | `input.body` | non-object JSON or plain text fallback |

---

## Pipelines

### GET /api/items — list, optional query filter

```
| trigger.webhook --path /api/programmes --method GET
| pg.query.run --credential my-pg --param "1={{ input.query.faculty_id ?? null }}" \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| script.result.run -- "return { ok: true, data: input.query.rows }"
```

### GET /api/items/:id — detail by path param

`:unit_id` in trigger path → `input.params.unit_id` → `$1`:

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query.run --credential my-pg --param "1={{ input.params.unit_id }}" \
    -- "SELECT au.unit_id::text, au.code, au.title, COUNT(DISTINCT s.student_id) as total_students FROM academic.academic_unit au LEFT JOIN academic.student s ON s.unit_id = au.unit_id AND s.is_active = true WHERE au.unit_id = $1::uuid AND au.unit_type = 'programme' GROUP BY au.unit_id, au.code, au.title"
| script.result.run -- "return { ok: true, data: input.query.rows[0] };"
(see **Answering with a status** below for the 404 branch)
```

### GET /api/items/:id — detail with related records (two pg.query.run nodes)

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query.run --credential my-pg --param "1={{ input.params.unit_id }}" \
    -- "SELECT unit_id::text, code, title FROM academic.academic_unit WHERE unit_id = $1::uuid"
| script.result.run -- "const prog = input.query.rows[0]; return { ...prog, unit_id: prog.unit_id };"
| pg.query.run --credential my-pg --param "1={{ input.unit_id }}" \
    -- "SELECT p.fullname, l.academic_rank, st.position FROM academic.lecturer l JOIN academic.staff st ON st.staff_id = l.staff_id JOIN app.player p ON p.player_id = st.player_id WHERE st.unit_id = $1::uuid AND l.is_active = true ORDER BY p.fullname"
| script.result.run -- "return { ok: true, data: { ...ctx.nodes['n2'], lecturers: input.query.rows } }"
```

Note: each `pg.query.run` REPLACES `input` with `{ query: { rows: [...] } }` — there is no
`input._prev`. To reach an earlier node's output after later nodes have
replaced the payload, use `ctx.nodes['<node id>']` in a script (pipe-mode node
ids are `n0`, `n1`, … in source order — the first `script` above is `n2`) or
`$nodes.<node id>` inside a `{{ }}` expression.

### POST /api/items — create from body

JSON body fields sit under `input.body`. Access as `input.body.title`, `input.body.email`, etc.:

```
| trigger.webhook --path /api/posts --method POST
| logic.if --expr "input.body.title && input.body.body"
(false pin → `web.response.send --status 400`; see **Answering with a status**)
| pg.query.run --credential my-pg --write --param "1={{ input.body.title }}" --param "2={{ input.body.body }}" --param "3={{ input.body.author_id }}" \
    -- "INSERT INTO posts (title, body, author_id, created_at) VALUES ($1, $2, $3, now()) RETURNING id, title"
| script.result.run -- "return { ok: true, data: input.query.rows?.[0] }"
```

### PUT /api/items/:id — update by path param + body

Combine path param and body fields with `--param`:

```
| trigger.webhook --path /api/posts/:id --method PUT
| pg.query.run --credential my-pg --write --param "1={{ input.body.title }}" --param "2={{ input.body.body }}" --param "3={{ input.params.id }}" \
    -- "UPDATE posts SET title = $1, body = $2, updated_at = now() WHERE id = $3 RETURNING id, title"
| script.result.run -- "return { ok: true, data: input.query.rows[0] };"
(see **Answering with a status** below for the 404 branch)
```

### DELETE /api/items/:id — delete by path param

```
| trigger.webhook --path /api/posts/:id --method DELETE
| pg.query.run --credential my-pg --write --param "1={{ input.params.id }}" \
    -- "DELETE FROM posts WHERE id = $1 RETURNING id"
| script.result.run -- "return { ok: true, deleted: input.query.rows?.[0]?.id ?? null }"
```

---

## `--param`: one placeholder per flag

| | One bind value | Several bind values |
|---|---|---|
| Best for | A single `$1` from a known path | Multiple placeholders, conditional values |
| Syntax | `--param "1={{ input.<dot.path> }}"` | `--param "1={{ a }}" --param "2={{ b }}"` (repeat the flag) |
| Typing | A whole `{{ expr }}` keeps its JSON type | Same, per key |
| Example | `--param "1={{ input.params.unit_id }}"` | `--param "1={{ input.body.title }}" --param "2={{ input.params.id }}"` |

---

## Answering with a status

A script cannot set the response. It returns a value; the graph decides what
happens next. To answer 404, branch and let `web.response.send` answer:

```
[find]  pg.query.run --credential my-pg --param "1={{ input.params.id }}" -- "SELECT …"
[found] logic.if --expr "input.query.rows && input.query.rows.length > 0"
[ok]    web.response.send --body "{{ { ok: true, data: input.query.rows[0] } }}"
[gone]  web.response.send --status 404 --body "{{ { ok: false, error: 'not found' } }}"
[find] -> [found]
[found]:true -> [ok]
[found]:false -> [gone]
```

`web.response.send` owns the response: `--status`, `--location`, `--set-cookie`,
`--body`, `--template`. The branch is visible in the editor, which a key hidden
in a payload would not be.

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints; path params in `input.params.<name>`, user-submitted data (JSON, form-urlencoded, multipart text fields) under `input.body.<name>`
- `pg.query.run --credential <id>` — parameterized SQL; `--param "1={{ input.path }}"` (one placeholder) or repeat `--param` for several, plus `--write` for INSERT/UPDATE/DELETE
- `script` — validation, 404 guard, response shaping, chaining multiple queries

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
