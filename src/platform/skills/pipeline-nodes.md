# Pipeline Node Reference

## Trigger Nodes

### `trigger_webhook`
Receives HTTP requests. The entry point for webhook pipelines.

```json
{
  "kind": "trigger_webhook",
  "config": {
    "path": "/api/posts",
    "method": "POST"
  }
}
```

Output: `{ method, path, query, headers, body }`

### `trigger_schedule`
Triggered by cron expression.

```json
{
  "kind": "trigger_schedule",
  "config": {
    "cron": "0 */6 * * *"
  }
}
```

## Processing Nodes

### `script`
Executes JavaScript. Receives the upstream output as `input`. Must return a value.

```json
{
  "kind": "script",
  "config": {
    "code": "const slug = input.body.title.toLowerCase().replace(/\\s+/g, '-');\nreturn { ...input.body, slug };"
  }
}
```

The script runs in a sandboxed JS runtime. Available globals: `JSON`, `Math`, `Date`, standard JS APIs.

### `http_request`
Makes an outbound HTTP request.

```json
{
  "kind": "http_request",
  "config": {
    "url": "https://api.example.com/data",
    "method": "GET",
    "headers": {"Authorization": "Bearer {{credentials.my_api_key.secret.token}}"}
  }
}
```

### `web_render`
Renders a TSX template with the upstream data as state. Returns HTML.

```json
{
  "kind": "web_render",
  "config": {
    "template_path": "pages/blog-home.tsx",
    "route": "/blog"
  }
}
```

**DSL flags:** `--template-id pages/blog-home.tsx --template-path pages/blog-home.tsx --route /blog`

**CRITICAL:** BOTH `template_id` AND `template_path` must be set to the same value. Missing either causes a runtime error. Always set both in every `web.render` DSL node or patch.

Fields:
- `template_id` (**required**) — used by the framework engine to identify the template
- `template_path` (**required**) — used by the platform to find the template file; same value as `template_id`
- `route` (**required**) — URL route passed to the render context, e.g. `/blog`

When the terminal node is `web_render`, the webhook response is HTML with `Content-Type: text/html`.

## Data Nodes

### `n.sekejap.query`
Queries or upserts rows in Sekejap — Zebflow's embedded multi-model database (graph, vector, spatial, full-text, temporal). Create tables in the UI (Tables page) before using.

**DSL name:** `sekejap.query`

**List rows:**
```json
{
  "kind": "n.sekejap.query",
  "config": {
    "operation": "query",
    "table": "blog_posts"
  }
}
```

**Get by field:**
```json
{
  "kind": "n.sekejap.query",
  "config": {
    "operation": "query",
    "table": "blog_posts",
    "where_field": "slug",
    "where_value_path": "/query/slug"
  }
}
```

**Upsert row:**
```json
{
  "kind": "n.sekejap.query",
  "config": {
    "operation": "upsert",
    "table": "blog_posts",
    "row_id_path": "/body/id",
    "data_path": "/body"
  }
}
```

### `trigger_ws`
Entry point for WebSocket pipelines. Receives client events from a WS room.

```json
{
  "kind": "n.trigger.ws",
  "config": {
    "room": "",
    "event": "chat"
  }
}
```

**DSL flags:** `--event <name> --room <room-id>`

- `event` — event name to match (empty = match all events)
- `room` — room id pattern to match (empty = match any room)

Output payload contains: `room_id`, `session_id`, `event`, `payload` (the client's message body).

**DSL name:** `trigger.ws`

---

### `ws_emit`
Broadcasts an event to connected clients in a WS room.

```json
{
  "kind": "n.ws.emit",
  "config": {
    "event": "chat",
    "to": "all",
    "payload_path": "/payload",
    "room": ""
  }
}
```

**DSL flags:** `--event <name> --to <all|session|others> --payload-path <json-pointer> --room <id>`

- `event` — event name sent to clients
- `to` — `"all"` (default), `"session"` (only triggering client), `"others"` (everyone except trigger)
- `payload_path` — JSON pointer into the upstream payload to extract as the event body (empty = whole payload)
- `room` — static room override (required for non-WS-triggered pipelines)

**DSL name:** `ws.emit`

---

### `ws_sync_state`
Mutates the shared room state and broadcasts a `state_patch` to all clients.

```json
{
  "kind": "n.ws.sync_state",
  "config": {
    "op": "merge",
    "path": "/players/{session_id}",
    "value_path": "/payload",
    "room": "",
    "silent": false
  }
}
```

**DSL flags:** `--op <set|merge|delete> --path <json-pointer> --value-path <json-pointer> --room <id> --silent`

- `op` — `"set"` (replace), `"merge"` (shallow merge), `"delete"` (remove key)
- `path` — JSON pointer destination; supports `{session_id}`, `{room_id}` placeholders
- `value_path` — pointer into the payload for the value to write (empty = entire payload)
- `room` — static room override
- `silent` — batch mutations for high-frequency streams (≥10 Hz)

**DSL name:** `ws.sync_state`

---

### `pg.query`

Executes a SQL query against a PostgreSQL credential. SELECT/WITH returns `{ rows: [...] }`. INSERT/UPDATE/DELETE returns `{ affected_rows: N }`.

**DSL:** `pg.query --credential <slug> [--params-path <dot.path>] [--params-expr <js>] -- "SQL"`

**DSL flags:**

| Flag | Description |
|------|-------------|
| `--credential <slug>` | **Required.** PostgreSQL credential slug from project credentials. |
| `--params-path <dot.path>` | Dot-notation path into upstream payload → value becomes `$1` (scalar) or `$1,$2,...` (array). e.g. `params.unit_id`, `query.status` |
| `--params-expr <js>` | JS expression evaluated against `input` → must return array `[$1, $2, ...]`. Use for multiple or conditional params. |

**Webhook input shape** (what's available in `--params-path` / `--params-expr`):
- `input.params.<name>` — path segment from `:name` in the trigger path (e.g. `--path /api/users/:id` → `input.params.id`)
- `input.query.<name>` — URL query string value (e.g. `?status=active` → `input.query.status`)
- `input.<name>` — GET: path params + query string are also merged to root; POST with JSON object body: fields merged to root
- `input.body` — raw body or nested non-object JSON

**IMPORTANT: `--params-path` uses dot notation**, not JSON pointer syntax. Use `params.unit_id` not `/params/unit_id`.

**Static query (no params):**
```
| pg.query --credential my-pg -- "SELECT id, title FROM posts ORDER BY created_at DESC LIMIT 20"
```

**Path param** — `:unit_id` in trigger path → `input.params.unit_id` → dot path `params.unit_id`:
```
| trigger.webhook --path /api/programmes/:unit_id --method GET
| pg.query --credential my-pg --params-path params.unit_id \
    -- "SELECT unit_id::text, code, title FROM academic_unit WHERE unit_id = $1::uuid"
```

**Query string param** — `?status=active` → `input.query.status` → dot path `query.status`:
```
| trigger.webhook --path /api/users --method GET
| pg.query --credential my-pg --params-path query.status \
    -- "SELECT id, name FROM users WHERE status = $1"
```

**Multiple params via JS expression:**
```
| trigger.webhook --path /api/search --method GET
| pg.query --credential my-pg --params-expr "[input.query.q, input.query.limit || '20']" \
    -- "SELECT * FROM posts WHERE title ILIKE '%' || $1 || '%' LIMIT $2::int"
```

**POST body fields:**
```
| trigger.webhook --path /api/users --method POST
| pg.query --credential my-pg --params-expr "[input.name, input.email]" \
    -- "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id"
```

**`--params-path` vs `--params-expr`:**

| | `--params-path` | `--params-expr` |
|---|---|---|
| Use when | Single `$1` from a known dot path | Multiple values or conditional logic |
| Syntax | Dot notation: `params.id`, `query.status` | JS expression: `[input.name, input.email]` |
| Returns | Scalar → `[$1]`; array → `[$1, $2, ...]` | Array → `[$1, $2, ...]` |
