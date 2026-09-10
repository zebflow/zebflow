# REST API + PostgreSQL

## What this builds

JSON REST API endpoints backed by PostgreSQL. Covers list, detail (path param), create, update, delete. Uses `--params` (dot path or `{{ }}` array expression) for safe parameterized queries — no string interpolation, no SQL injection risk.

---

## Key Concept: Parameter Binding

Path params (`:id`), query strings (`?status=x`), and body fields all land in the webhook input payload. Bind them safely to SQL `$1`, `$2`, ... via:

- `--params "{{ input.params.id }}"` — single value from a dot path → becomes `$1`
- `--params "{{ [input.name, input.email] }}"` — array expression → becomes `$1, $2, ...`

**Webhook input shape:**

| Location | Access | Example |
|----------|--------|---------|
| Path param `:id` | `input.params.id` | `--path /api/users/:id` |
| Query string `?status=x` | `input.query.status` | `?status=active` |
| GET root (merged) | `input.id` | path params + query merged to root for GET |
| POST JSON body field | `input.name` | `application/json` object fields merged to root |
| POST form field | `input.name` | `application/x-www-form-urlencoded` fields merged to root (percent-decoded) |
| POST multipart text field | `input.name` | `multipart/form-data` text fields merged to root |
| POST multipart file | `input.files.avatar` | `{ filename, content_type, size, data }` — data is base64 |
| Raw / non-object body | `input.body` | non-object JSON or plain text fallback |

---

## Pipelines

### GET /api/items — list, optional query filter

```
| trigger.webhook --path /api/programmes --method GET
| pg.query --credential my-pg --params "{{ [input.query.faculty_id ?? null] }}" \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| script -- "return { ok: true, data: input.rows }"
```

### GET /api/items/:id — detail by path param

`:unit_id` in trigger path → `input.params.unit_id` → `$1`:

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query --credential my-pg --params "{{ input.params.unit_id }}" \
    -- "SELECT au.unit_id::text, au.code, au.title, COUNT(DISTINCT s.student_id) as total_students FROM academic.academic_unit au LEFT JOIN academic.student s ON s.unit_id = au.unit_id AND s.is_active = true WHERE au.unit_id = $1::uuid AND au.unit_type = 'programme' GROUP BY au.unit_id, au.code, au.title"
| script -- "return { ok: true, data: input.rows[0] };"
(see **Answering with a status** below for the 404 branch)
```

### GET /api/items/:id — detail with related records (two pg.query nodes)

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query --credential my-pg --params "{{ input.params.unit_id }}" \
    -- "SELECT unit_id::text, code, title FROM academic.academic_unit WHERE unit_id = $1::uuid"
| script -- "const prog = input.rows[0]; return { ...prog, unit_id: prog.unit_id };"
| pg.query --credential my-pg --params "{{ input.unit_id }}" \
    -- "SELECT p.fullname, l.academic_rank, st.position FROM academic.lecturer l JOIN academic.staff st ON st.staff_id = l.staff_id JOIN app.player p ON p.player_id = st.player_id WHERE st.unit_id = $1::uuid AND l.is_active = true ORDER BY p.fullname"
| script -- "return { ok: true, data: { ...input._prev, lecturers: input.rows } }"
```

Note: each pg.query replaces `input` with `{ rows: [...] }`. Use a script node to carry forward fields between queries by merging into a running context.

### POST /api/items — create from body

JSON body fields are merged to root for object bodies. Access as `input.name`, `input.email`, etc.:

```
| trigger.webhook --path /api/posts --method POST
| logic.if --expr "input.title && input.body"
(false pin → `web.response --status 400`; see **Answering with a status**)
| pg.query --credential my-pg --params "{{ [input.title, input.body, input.author_id] }}" \
    -- "INSERT INTO posts (title, body, author_id, created_at) VALUES ($1, $2, $3, now()) RETURNING id, title"
| script -- "return { ok: true, data: input.rows?.[0] }"
```

### PUT /api/items/:id — update by path param + body

Combine path param and body fields with `--params`:

```
| trigger.webhook --path /api/posts/:id --method PUT
| pg.query --credential my-pg --params "{{ [input.title, input.body, input.params.id] }}" \
    -- "UPDATE posts SET title = $1, body = $2, updated_at = now() WHERE id = $3 RETURNING id, title"
| script -- "return { ok: true, data: input.rows[0] };"
(see **Answering with a status** below for the 404 branch)
```

### DELETE /api/items/:id — delete by path param

```
| trigger.webhook --path /api/posts/:id --method DELETE
| pg.query --credential my-pg --params "{{ input.params.id }}" \
    -- "DELETE FROM posts WHERE id = $1 RETURNING id"
| script -- "return { ok: true, deleted: input.rows?.[0]?.id ?? null }"
```

---

## `--params`: single value vs array expression

| | Single value | Array expression |
|---|---|---|
| Best for | Single `$1` from a known path | Multiple bind values, type coercion, conditional |
| Syntax | `--params "{{ input.<dot.path> }}"` | `--params "{{ [input.title, input.email] }}"` |
| Scalar result | Wrapped as `[$1]` | Must return array explicitly |
| Example | `--params "{{ input.params.unit_id }}"` | `--params "{{ [input.title, input.params.id] }}"` |

---

## Answering with a status

A script cannot set the response. It returns a value; the graph decides what
happens next. To answer 404, branch and let `web.response` answer:

```
[find]  pg.query --credential my-pg --params "{{ input.params.id }}" -- "SELECT …"
[found] logic.if --expr "input.rows && input.rows.length > 0"
[ok]    web.response --body "{{ { ok: true, data: input.rows[0] } }}"
[gone]  web.response --status 404 --body "{{ { ok: false, error: 'not found' } }}"
[find] -> [found]
[found]:true -> [ok]
[found]:false -> [gone]
```

`web.response` owns the response: `--status`, `--location`, `--set-cookie`,
`--body`, `--template`. The branch is visible in the editor, which a key hidden
in a payload would not be.

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints; path params in `input.params.<name>`, body merged to root for JSON, form-urlencoded, and multipart text fields
- `pg.query` — parameterized SQL; `--params "{{ input.path }}"` (single value) or `--params "{{ [a, b] }}"` (multiple/conditional)
- `script` — validation, 404 guard, response shaping, chaining multiple queries

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
