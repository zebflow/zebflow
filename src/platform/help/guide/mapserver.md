# MapServer

MapServer is Zebflow's first-class geospatial publishing and serving surface.

Its job is not only storing geodata. Its job is:

- publishing layers
- resolving spatial requests efficiently
- serving map-facing responses

## What it pairs with

- project files such as GeoJSON and GeoParquet under `mapserver/`
- chunked, published artifacts — a fast spatial index built from a large
  GeoJSON source, cached under `data/cache/mapserver-artifacts/{instance}/{layer}/`
  and rebuilt from the uploaded source when missing
- TSX pages using `zeb/deckgl`

## Publishing and serving

Layers are managed with the `n.ms.publish` / `n.ms.unpublish` / `n.ms.get` /
`n.ms.list` pipeline nodes (see `help("pipeline/dsl")` for flags) and through
`/api/projects/{owner}/{project}/mapserver/{instance}/sources` and
`.../layers`. A published layer is immediately queryable at
`/ms/{owner}/{project}/{path}`.

## Why it matters

It turns geospatial data into project-native application behavior.

Typical flow:

1. publish a layer with `n.ms.publish` (or the mapserver API)
2. query it by viewport / filters at `/ms/{owner}/{project}/{path}`
3. render it in a page or map experience with `zeb/deckgl`

MapServer is treated as a first-class capability in Zebflow, not as a plugin.
