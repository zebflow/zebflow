# Pipeline Language

Pipeline authoring is a user surface. Use this page for the public language. Internal parsing, activation, and runtime behavior belong in `docs/developer/pipeline-runtime.md`.

Detailed live reference:

- `help(topic="pipeline")`
- `help(topic="pipeline/dsl")`
- `help(topic="pipeline/nodes")`
- `help(topic="pipeline/nodes/{kind}")`

Source authority:

- `src/platform/help/pipeline/index.md`
- `src/platform/help/pipeline/dsl.md`
- `src/pipeline/model.rs`
- `src/pipeline/nodes/basic/`

## Formats

| Format | Use when |
|---|---|
| Pipe DSL | Linear workflows. |
| Graph DSL | Branching, named pins, and non-linear flow. |
| JSON graph | Exact stored pipeline form in `.zf.json`. |

## Pipe DSL

```zf
| trigger.webhook --path /blog --method GET
| sekejap.query -- "SELECT * FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx --route /blog
```

## Graph DSL

```zf
[a] trigger.webhook --path /ingest --method POST
[b] logic.match --expr "input.kind" --cases csv,geojson --default other
[c] table.convert --from-expr "input.files.uploaded.path" --from-format csv --to data/out.parquet --to-format parquet
[d] geo.convert --from-expr "input.files.uploaded.path" --to data/out.parquet --to-format geoparquet
[e] script -- "return { ok: false, error: 'unsupported kind' }"
[a] -> [b]
[b]:csv -> [c]
[b]:geojson -> [d]
[b]:other -> [e]
```

## JSON Graph

```json
{
  "nodes": [
    { "id": "a", "kind": "n.trigger.webhook", "config": { "path": "/blog", "method": "GET" } },
    { "id": "b", "kind": "n.web.response", "config": { "template": "pages/blog-home.tsx" } }
  ],
  "edges": [
    { "from_node": "a", "from_pin": "out", "to_node": "b", "to_pin": "in" }
  ]
}
```

## Data Model

- `input` is the payload passed from upstream nodes.
- `ctx` is run context such as request id, pipeline id, trigger metadata, auth, params, query, and headers.
- Edges carry payloads.
- Pins decide which edge is taken.
- Node definitions define valid config, pins, schemas, and documentation.

## FileRef

Uploaded files and stored artifacts should pass as file references, not inline bytes:

```json
{
  "files": {
    "uploaded": {
      "path": "tmp/upload.csv",
      "url": "/fs/superadmin/default/tmp/upload.csv",
      "size": 228,
      "content_type": "text/csv"
    }
  }
}
```

Use file/table/geo nodes to read or convert large data instead of putting whole datasets in JSON payloads.
