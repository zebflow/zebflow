# REST API + PostgreSQL

## What this builds

JSON REST API endpoints backed by PostgreSQL. Covers list, detail (path param), create, update, delete. Uses `--params-path` and `--params-expr` for safe parameterized queries — no string interpolation, no SQL injection risk.

---

## Key Concept: Parameter Binding

Path params (`:id`), query strings (`?status=x`), and body fields all land in the webhook input payload. Bind them safely to SQL `$1`, `$2`, ... via:

- `--params-path params.id` — JSON pointer to a single value → becomes `$1`
- `--params-expr "[input.name, input.email]"` — JS expression → array becomes `$1, $2, ...`

**Webhook input shape:**

| Location | Access | Example |
|----------|--------|---------|
| Path param `:id` | `input.params.id` | `--path /api/users/:id` |
| Query string `?status=x` | `input.query.status` | `?status=active` |
| GET root (merged) | `input.id` | path params + query merged to root for GET |
| POST JSON body field | `input.name` | JSON object body fields merged to root |
| POST raw/nested body | `input.body` | non-object JSON or text |

---

## Pipelines

### GET /api/items — list, optional query filter

```
| trigger.webhook --path /api/programmes --method GET
| pg.query --credential my-pg --params-expr "[input.query.faculty_id ?? null]" \
    -- "SELECT unit_id::text, code, title->>'id' as title, slug FROM academic.academic_unit WHERE unit_type = 'programme' AND is_active = true AND ($1::uuid IS NULL OR parent_unit_id = $1::uuid) ORDER BY code"
| script -- "return { ok: true, data: input.rows }"
```

### GET /api/items/:id — detail by path param

`:unit_id` in trigger path → `input.params.unit_id` → `$1`:

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query --credential my-pg --params-path params.unit_id \
    -- "SELECT au.unit_id::text, au.code, au.title, COUNT(DISTINCT s.student_id) as total_students FROM academic.academic_unit au LEFT JOIN academic.student s ON s.unit_id = au.unit_id AND s.is_active = true WHERE au.unit_id = $1::uuid AND au.unit_type = 'programme' GROUP BY au.unit_id, au.code, au.title"
| script -- "const r = input.rows?.[0]; if (!r) return { __status: 404, error: 'not found' }; return { ok: true, data: r }"
```

### GET /api/items/:id — detail with related records (two pg.query nodes)

```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query --credential my-pg --params-path params.unit_id \
    -- "SELECT unit_id::text, code, title FROM academic.academic_unit WHERE unit_id = $1::uuid"
| script -- "const prog = input.rows?.[0]; if (!prog) return { __status: 404 }; return { ...prog, unit_id: prog.unit_id }"
| pg.query --credential my-pg --params-path unit_id \
    -- "SELECT p.fullname, l.academic_rank, st.position FROM academic.lecturer l JOIN academic.staff st ON st.staff_id = l.staff_id JOIN app.player p ON p.player_id = st.player_id WHERE st.unit_id = $1::uuid AND l.is_active = true ORDER BY p.fullname"
| script -- "return { ok: true, data: { ...input._prev, lecturers: input.rows } }"
```

Note: each pg.query replaces `input` with `{ rows: [...] }`. Use a script node to carry forward fields between queries by merging into a running context.

### POST /api/items — create from body

JSON body fields are merged to root for object bodies. Access as `input.name`, `input.email`, etc.:

```
| trigger.webhook --path /api/posts --method POST
| script -- "if (!input.title || !input.body) return { __status: 400, error: 'title and body required' }; return input"
| pg.query --credential my-pg --params-expr "[input.title, input.body, input.author_id]" \
    -- "INSERT INTO posts (title, body, author_id, created_at) VALUES ($1, $2, $3, now()) RETURNING id, title"
| script -- "return { ok: true, data: input.rows?.[0] }"
```

### PUT /api/items/:id — update by path param + body

Combine path param and body fields with `--params-expr`:

```
| trigger.webhook --path /api/posts/:id --method PUT
| pg.query --credential my-pg --params-expr "[input.title, input.body, input.params.id]" \
    -- "UPDATE posts SET title = $1, body = $2, updated_at = now() WHERE id = $3 RETURNING id, title"
| script -- "const r = input.rows?.[0]; if (!r) return { __status: 404, error: 'not found' }; return { ok: true, data: r }"
```

### DELETE /api/items/:id — delete by path param

```
| trigger.webhook --path /api/posts/:id --method DELETE
| pg.query --credential my-pg --params-path params.id \
    -- "DELETE FROM posts WHERE id = $1 RETURNING id"
| script -- "return { ok: true, deleted: input.rows?.[0]?.id ?? null }"
```

---

## `--params-path` vs `--params-expr`

| | `--params-path` | `--params-expr` |
|---|---|---|
| Best for | Single `$1` from a known path | Multiple bind values, type coercion, conditional |
| Syntax | Dot notation: `params.id`, `query.status` | JS expression: `[input.title, input.email]` |
| Scalar result | Wrapped as `[$1]` | Must return array explicitly |
| Example | `--params-path params.unit_id` | `--params-expr "[input.title, input.params.id]"` |

---

## Special Script Output Keys

| Key | Effect |
|-----|--------|
| `__status` | Set HTTP response status code (400, 404, 401, etc.) |
| `_redirect` | Redirect response to a URL |
| `_set_cookie` | Set an HttpOnly cookie `{ name, value, http_only, max_age, path }` |

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints; path params in `input.params.<name>`, body merged to root for POST JSON
- `pg.query` — parameterized SQL; `--params-path` (single value) or `--params-expr` (multiple/conditional)
- `script` — validation, 404 guard, response shaping, chaining multiple queries
