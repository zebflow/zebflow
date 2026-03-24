# zeb/deckgl

Deck.gl 9.x for geospatial and large-scale data visualisation in RWE templates.
Fully offline — `@deck.gl/core` + `@deck.gl/layers` bundled inline, no CDN.
Runs on a plain WebGL canvas — no map backend (Mapbox/MapLibre) required.

## Import

```tsx
import DeckMap from "zeb/deckgl";
```

---

## `DeckMap` Component

```tsx
<DeckMap
  id="my-map"
  height="400px"
  initialViewState={{ longitude: -74, latitude: 40.7, zoom: 10 }}
  layers={[{ type: "ScatterplotLayer", data: points, getPosition: "position", getRadius: 80 }]}
  controller={true}
  stateKey="mapView"
  layerKey="mapData"
  background="transparent"
  className="rounded-lg overflow-hidden"
/>
```

### Props

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `id` | `string` | auto | Container id — required for `window.__zebDeck.get(id)` |
| `height` | `string` | `"400px"` | CSS height of the canvas container |
| `className` | `string` | — | Tailwind classes on the container |
| `initialViewState` | `object` | world view | `{ longitude, latitude, zoom, pitch, bearing }` |
| `controller` | `boolean` | `true` | Enable pan/zoom/rotate interaction |
| `layers` | `LayerConfig[]` | `[]` | Declarative layer specs (see below) |
| `stateKey` | `string` | — | Page state key for two-way view state sync |
| `layerKey` | `string` | — | Page state key → layer data array (auto-builds ScatterplotLayer) |
| `background` | `string` | `"transparent"` | Canvas clear colour |

---

## Declarative layers

Pass layer config objects with a `type` string. Accessor props accept either
direct values or simple string keys (`"position"` → `d => d.position`).

```tsx
layers={[
  {
    type: "ScatterplotLayer",
    id: "points",
    data: myPoints,         // array of objects
    getPosition: "coords",  // d => d.coords
    getFillColor: [0, 180, 255, 200],
    getRadius: 80,
    radiusScale: 1,
    pickable: true,
  },
  {
    type: "ArcLayer",
    id: "connections",
    data: myArcs,
    getSourcePosition: "source",  // d => d.source
    getTargetPosition: "target",  // d => d.target
    getSourceColor: [0, 200, 255],
    getTargetColor: [255, 100, 0],
    getWidth: 2,
  },
]}
```

### String accessor syntax

| Spec | Resolves to |
|------|------------|
| `"position"` | `d => d.position` |
| `"[lon, lat]"` | `d => [d.lon, d.lat]` |
| `"[0]"` | `d => d[0]` |

For complex accessors (computed values, conditionals), use imperative access via
`zeb:deck:ready` event or `window.__zebDeck.get(id)`.

### Available layer types

`ScatterplotLayer`, `LineLayer`, `ArcLayer`, `PathLayer`, `PolygonLayer`,
`SolidPolygonLayer`, `IconLayer`, `TextLayer`, `ColumnLayer`, `GridCellLayer`,
`PointCloudLayer`, `BitmapLayer`

---

## Patterns

### API-first: data loads after fetch

```tsx
const [points, setPoints] = usePageState("mapPoints", []);

useEffect(() => {
  fetch("/api/locations").then(r => r.json()).then(setPoints);
}, []);

<DeckMap
  height="500px"
  initialViewState={{ longitude: 103.8, latitude: 1.35, zoom: 11 }}
  layerKey="mapPoints"
/>
```

When `setPoints(data)` fires `rwe:state:change`, the DeckMap's listener rebuilds
the layers from the new data automatically.

### Two-way view state sync

```tsx
const [view, setView] = usePageState("mapView", { longitude: 0, latitude: 20, zoom: 2 });

<DeckMap
  stateKey="mapView"
  height="400px"
/>
<p>Zoom: {view.zoom?.toFixed(1)}</p>
```

When the user pans/zooms, `view` updates via page state. When `setView(...)` is
called from elsewhere, the map flies to the new position.

### Multiple maps sharing a view

```tsx
const [view] = usePageState("sharedView", { longitude: 0, latitude: 0, zoom: 2 });

<DeckMap id="map-a" stateKey="sharedView" layerKey="layersA" height="300px" />
<DeckMap id="map-b" stateKey="sharedView" layerKey="layersB" height="300px" />
```

Both maps stay in sync as the user pans either one.

---

## Events

### `zeb:deck:ready`

Fires once when the Deck.gl instance finishes mounting.

```tsx
document.getElementById("my-map").addEventListener("zeb:deck:ready", (e) => {
  const { deck, instance, id } = e.detail;

  // Set complex layers with accessor functions:
  instance.setLayers([
    new ScatterplotLayer({
      data: myPoints,
      getPosition: d => [d.lon, d.lat],
      getFillColor: d => d.active ? [0, 255, 100] : [150, 150, 150],
      getRadius: d => Math.sqrt(d.value) * 10,
    }),
  ]);
});
```

---

## Imperative API — `window.__zebDeck`

```ts
const inst = window.__zebDeck.get("my-map");  // DeckInstance | undefined

inst.deck                // raw Deck.gl Deck instance
inst.setLayers(layers)   // replace layers — accepts Layer instances or JSON configs
inst.setViewState(vs)    // fly to new view state
inst.destroy()           // finalize + remove from registry
```

### Raw deck.gl access

```tsx
// All deck.gl classes available on the deckgl namespace export:
import { deckgl } from "zeb/deckgl";
// deckgl.ScatterplotLayer, deckgl.ArcLayer, deckgl.Deck, etc.

// Or via window (set by the bundle):
window.__zebDeck.deck.ScatterplotLayer
```

### `buildLayer` / `buildLayers`

```tsx
import { buildLayer, buildLayers } from "zeb/deckgl";

const layer = buildLayer({
  type: "ScatterplotLayer",
  data: points,
  getPosition: "position",
  getRadius: 100,
});
```

---

## Direct mount — `mountDeckMap`

```tsx
import { mountDeckMap } from "zeb/deckgl";

const container = document.getElementById("my-div");
const runtime = mountDeckMap(container, {
  initialViewState: { longitude: -74, latitude: 40.7, zoom: 10 },
  layers: [new ScatterplotLayer({ ... })],
  controller: true,
});

runtime.setLayers([...]);   // replace layers
runtime.setViewState({...}); // update view
runtime.destroy();           // clean up
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Packages | `@deck.gl/core` + `@deck.gl/layers` |
| Bundle | `runtime/deckgl.bundle.mjs` (~850 KB minified) |
| CDN fetches | **None** — fully offline |
| Map backend | Not required (standalone WebGL canvas) |
| Build tool | esbuild |

### Rebuild

```sh
cd /tmp/zeb-deckgl-build
cp libraries/zeb/deckgl/0.1/runtime/entry.mjs .
cp libraries/zeb/deckgl/0.1/runtime/package.json .
npm install
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --minify \
  --outfile=deckgl.bundle.mjs
cp deckgl.bundle.mjs libraries/zeb/deckgl/0.1/runtime/
cargo build
```
