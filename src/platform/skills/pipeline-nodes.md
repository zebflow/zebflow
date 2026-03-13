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

**DSL flags:** `--template-path pages/blog-home.tsx --route /blog`

**CRITICAL:** The field is `template_path` (full path **with** `.tsx` extension). Do NOT use `--template` or `--template-id` — both silently store a wrong field and the node will fail at runtime.

Fields:
- `template_path` (**required**) — full path to template file **including** `.tsx`, e.g. `pages/blog-home.tsx`
- `route` (**required**) — URL route passed to the render context, e.g. `/blog`

When the terminal node is `web_render`, the webhook response is HTML with `Content-Type: text/html`.

## Data Nodes

### `sjtable_query`
Queries or mutates a Simple Table (Sekejap-backed).

**List rows:**
```json
{
  "kind": "sjtable_query",
  "config": {
    "operation": "list",
    "table": "blog_posts"
  }
}
```

**Get by field:**
```json
{
  "kind": "sjtable_query",
  "config": {
    "operation": "get",
    "table": "blog_posts",
    "where_field": "slug",
    "where_value": "{{input.query.slug}}"
  }
}
```

**Upsert row:**
```json
{
  "kind": "sjtable_query",
  "config": {
    "operation": "upsert",
    "table": "blog_posts",
    "row_id": "{{input.body.id}}",
    "data": "{{input.body}}"
  }
}
```

**Delete row:**
```json
{
  "kind": "sjtable_query",
  "config": {
    "operation": "delete",
    "table": "blog_posts",
    "row_id": "{{input.body.id}}"
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

### `pg_query`
Executes a SQL query against a PostgreSQL DB connection.

```json
{
  "kind": "pg_query",
  "config": {
    "connection_slug": "my_pg_connection",
    "sql": "SELECT * FROM posts WHERE status = $1",
    "params": ["published"]
  }
}
```
