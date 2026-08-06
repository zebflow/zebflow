# Pipeline Examples

Detailed examples also exist in `src/platform/help/pipeline/examples/` and are available through `help(topic="pipeline/examples")`.

## Web Page

```zf
| trigger.webhook --path /blog --method GET
| sekejap.query -- "SELECT * FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx --route /blog
```

## JSON API

```zf
| trigger.webhook --path /api/posts --method POST
| script -- "return { title: input.title, body: input.body, created_at: Date.now() }"
| sekejap.insert --items-expr "[input]"
| script -- "return { ok: true }"
```

## File Upload

```zf
| trigger.webhook --path /api/upload --method POST
| fs.save --field uploaded --folder uploads --allowed-kinds csv,json,images
| script -- "return { ok: true, file: input.saved }"
```

## CSV to Parquet

```zf
| trigger.webhook --path /api/csv --method POST
| table.convert --from-expr "input.files.uploaded.path" --from-format csv --to data/out.parquet --to-format parquet
```

## Publish Map Layer

```zf
| trigger.manual
| ms.publish --name parks --path /maps/parks --source-path data/parks.parquet --source-kind geoparquet
```
