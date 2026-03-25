# Pipeline Authoring

Pipelines are JSON-defined directed graphs stored as `.zf.json` files.

## File Format

```json
{
  "nodes": [
    {
      "id": "n1",
      "kind": "trigger_webhook",
      "config": {
        "path": "/api/hello",
        "method": "GET"
      },
      "pins_out": [{"id": "out", "label": "output"}]
    },
    {
      "id": "n2",
      "kind": "script",
      "config": {
        "code": "return { message: 'Hello, world!' };"
      },
      "pins_in": [{"id": "in", "label": "input"}],
      "pins_out": [{"id": "out", "label": "output"}]
    }
  ],
  "edges": [
    {"from_node": "n1", "from_pin": "out", "to_node": "n2", "to_pin": "in"}
  ]
}
```

## Pipeline Lifecycle

1. **Create** — POST source JSON to the pipeline API
2. **Edit** — PUT updated source JSON
3. **Activate** — POST to `/activate` to promote to runtime
4. **Deactivate** — POST to `/deactivate` to remove from runtime

Only **activated** pipelines receive live webhook traffic.

## Pipeline API

```
GET    /api/projects/{o}/{p}/pipelines              — list registry
POST   /api/projects/{o}/{p}/pipelines              — create pipeline
GET    /api/projects/{o}/{p}/pipelines/file?...     — read pipeline source
PUT    /api/projects/{o}/{p}/pipelines/file         — update pipeline source
DELETE /api/projects/{o}/{p}/pipelines/file         — delete pipeline
POST   /api/projects/{o}/{p}/pipelines/activate     — activate pipeline
POST   /api/projects/{o}/{p}/pipelines/deactivate   — deactivate pipeline
POST   /api/projects/{o}/{p}/pipelines/execute      — manually execute
```

## Trigger Kinds

- `trigger_webhook` — triggered by HTTP webhook request
- `trigger_schedule` — triggered by cron schedule
- `trigger_function` — triggered by manual API call

## Webhook Ingress

When activated, webhook pipelines receive traffic at:
```
{method} /wh/{owner}/{project}/{configured-path}
```

Example: pipeline with `"path": "/blog"` and `"method": "GET"` receives:
```
GET /wh/alice/myproject/blog
```
