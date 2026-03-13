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
    "template_id": "pages/blog-home",
    "route": "/blog"
  }
}
```

**DSL flags:** `--template-id pages/blog-home --route /blog`

**CRITICAL:** The field is `template_id`, NOT `template`. Using `--template` will cause `missing field 'template_id'` at runtime.

Fields:
- `template_id` (**required**) — path to template file without `.tsx` extension, e.g. `pages/blog-home`
- `route` (**required**) — URL route passed to the render context, e.g. `/blog`
- `markup` (optional) — inline TSX string, used for direct graph execution only

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
