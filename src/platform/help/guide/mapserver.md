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
- TSX pages using `zeb/world` (`data.mvt("/ms/{owner}/{project}/{route}/{z}/{x}/{y}.mvt")`)
  or `zeb/deckgl`

## Publishing and serving

Layers are managed with the `mapserver.layer.publish` / `mapserver.layer.unpublish` / `mapserver.layer.get` /
`mapserver.layer.list` pipeline nodes and through
`/api/projects/{owner}/{project}/mapserver/{instance}/sources` and
`.../layers`. A published layer answers at `/ms/{owner}/{project}/{route}`
while the project's `ms` surface is on (Settings → Addressing).

```
| geo.dataset.convert --from uploads/suburbs.shp --folder datasets --filename suburbs.parquet --crs EPSG:4326
| mapserver.layer.publish --name suburbs --route suburbs --from "{{ input.dataset }}" --field name --field postcode --min-zoom 8 --max-zoom 14
```

- `--from` is a GeoJSON or GeoParquet store key or FileRef; `--parse
  geojson|geoparquet` says which when the name does not. It is served from an
  optimized GeoParquet copy under `mapserver/.optimized/` unless
  `--skip-optimize`; `--skip-optimize --build-artifact` serves a GeoJSON as
  chunks instead. A layer is served in longitude/latitude: a source in
  metres (EPSG:3857, a national grid) is refused at publish — reproject it
  with `geo.dataset.convert --crs EPSG:4326` and publish what that writes.
- `--function <pipeline>` publishes what a function pipeline returns (a
  GeoJSON FeatureCollection) instead of a file, cached for `--ttl` (`30s`,
  `5m`; default 60s).
- Each `--field` is a property the public may see; none serves geometry only.
  `--max-items` caps the features one query answers (default 1000).
- Vector tiles answer at `{route}/{z}/{x}/{y}.mvt` (or `.pbf`), one MVT layer named after the
  layer. A tile's feature carries an id only when the source feature has a non-negative integer
  `id` of its own; nothing is numbered per tile, so a client never mistakes two features of
  neighbouring tiles for one.
- Every node answers `layer`: publish `{ name, route, store, source,
  source_kind, …, serving }`, get `{ found, … }`, unpublish `{ name, removed
  }`, list `{ items, count }`.

## Why it matters

It turns geospatial data into project-native application behavior.

Typical flow:

1. publish a layer with `mapserver.layer.publish` (or the mapserver API)
2. query it by viewport / filters at `/ms/{owner}/{project}/{path}`
3. render it in a page or map experience with `zeb/world` or `zeb/deckgl`

## In the Studio

Databases → the mapserver connection → Layers lists the published layers; a layer opens its
stats, its attribute columns and a preview map. The map draws the layer from its own vector
tiles (`{route}/{z}/{x}/{y}.mvt`, within the layer's zoom range) with `zeb/world`, over the
earth-lite basemap that ships with Zebflow, so it asks nothing from outside: areas blue, lines
yellow, points green, and a feature's public properties on hover. The view is fitted to a
sample of up to 500 of the layer's features. The tiles answer only while the project's `ms`
surface is on.

MapServer is treated as a first-class capability in Zebflow, not as a plugin.
