# zeb/deckgl

Deck.gl 9.x for geospatial and large-scale data visualisation in RWE templates.
Fully offline — `@deck.gl/core` + `@deck.gl/layers` + `@deck.gl/aggregation-layers` +
`@deck.gl/geo-layers` + `@deck.gl/mesh-layers` + `@deck.gl/extensions` bundled inline, no CDN.
Runs on a plain WebGL canvas — no map backend (Mapbox/MapLibre) required.

## Import

```tsx
import DeckMap from "zeb/deckgl";
```

Named exports:

```tsx
import {
  deckgl,                // full deck.gl namespace (all classes)
  buildLayer,            // JSON config → Layer instance
  buildLayers,           // array of configs → array of Layer instances
  mountDeckMap,          // imperative mount
  createDeckMapRuntime,  // alias for mountDeckMap
  ensureDeck,            // returns deck namespace object
  haversine,             // distance between two [lon, lat] points (meters)
  bearing,               // bearing from point A to B (degrees)
  colorRamp,             // interpolate color stops at position t
  interpolateAlongPath,  // [lon, lat, heading] at progress along a path
  createAnimationLoop,   // rAF loop with play/pause/speed
  DeckMap,               // Preact component (same as default export)
} from "zeb/deckgl";
```

---

## `DeckMap` Component

```tsx
<DeckMap
  id="my-map"
  height="400px"
  initialViewState={{ longitude: -74, latitude: 40.7, zoom: 10 }}
  layers={[{ type: "ScatterplotLayer", data: points, getPosition: "position", getRadius: 80, pickable: true }]}
  controller={true}
  tooltip={true}
  stateKey="mapView"
  layerKey="mapData"
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
| `layerKey` | `string` | — | Page state key -> layer data array (auto-builds ScatterplotLayer) |
| `tooltip` | `boolean` | `false` | Enable default hover tooltip (shows object properties) |
| `background` | `string` | `"transparent"` | Canvas clear colour |

---

## Available Layer Types

### Basic Layers

`ScatterplotLayer`, `LineLayer`, `ArcLayer`, `PathLayer`, `PolygonLayer`,
`SolidPolygonLayer`, `IconLayer`, `TextLayer`, `ColumnLayer`, `GridCellLayer`,
`PointCloudLayer`, `BitmapLayer`

### Aggregation Layers

`HeatmapLayer`, `HexagonLayer`, `GridLayer`, `ContourLayer`, `ScreenGridLayer`

### Geo Layers

`GeoJsonLayer`, `TileLayer`, `MVTLayer`

### Mesh Layers

`ScenegraphLayer`, `SimpleMeshLayer`

### Extensions

`PathStyleExtension`, `DataFilterExtension`, `BrushingExtension`, `CollisionFilterExtension`

---

## Declarative Layers

Pass layer config objects with a `type` string. Accessor props accept either
direct values or simple string keys (`"position"` -> `d => d.position`).

```tsx
layers={[
  {
    type: "ScatterplotLayer",
    id: "points",
    data: myPoints,
    getPosition: "coords",
    getFillColor: [0, 180, 255, 200],
    getRadius: 80,
    pickable: true,
  },
  {
    type: "HeatmapLayer",
    id: "density",
    data: incidents,
    getPosition: "location",
    getWeight: "severity",
    radiusPixels: 60,
    intensity: 2,
  },
  {
    type: "GeoJsonLayer",
    id: "routes",
    data: geojson,
    getLineColor: [0, 150, 255],
    getLineWidth: 3,
  },
  {
    type: "PathLayer",
    id: "trails",
    data: trails,
    getPath: "coordinates",
    getColor: [255, 100, 0],
    getDashArray: [8, 4],
    extensions: [{ type: "PathStyleExtension", dash: true }],
  },
]}
```

### String accessor syntax

| Spec | Resolves to |
|------|------------|
| `"position"` | `d => d.position` |
| `"[lon, lat]"` | `d => [d.lon, d.lat]` |
| `"[0]"` | `d => d[0]` |

### TileLayer with OSM basemap

```tsx
{
  type: "TileLayer",
  data: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
  minZoom: 0,
  maxZoom: 19,
  tileSize: 256,
  renderSubLayers: "bitmap",  // shorthand for BitmapLayer sub-layers
}
```

For complex accessors (computed values, conditionals), use imperative access via
`zeb:deck:ready` event or `window.__zebDeck.get(id)`.

---

## Utility Functions

### `haversine(a, b)`

Distance in meters between two `[longitude, latitude]` points.

```tsx
import { haversine } from "zeb/deckgl";
const dist = haversine([101.68, 3.14], [101.70, 3.16]);
```

### `bearing(a, b)`

Bearing in degrees (0-360) from point A to point B.

```tsx
import { bearing } from "zeb/deckgl";
const deg = bearing([101.68, 3.14], [101.70, 3.16]);
```

### `colorRamp(t, stops)`

Interpolate a color ramp at position `t` (0-1). Returns `[r, g, b, a]`.

```tsx
import { colorRamp } from "zeb/deckgl";

// Named presets: "green-red", "blue-red", "cool", "warm", "viridis"
const color = colorRamp(0.7, "green-red");

// Custom stops:
const color2 = colorRamp(0.5, [
  [0,   [0, 100, 255]],
  [0.5, [255, 255, 0]],
  [1,   [255, 0, 0]],
]);
```

### `interpolateAlongPath(path, progress)`

Returns `[longitude, latitude, heading]` at progress (0-1) along a coordinate array.

```tsx
import { interpolateAlongPath } from "zeb/deckgl";
const path = [[101.68, 3.14], [101.69, 3.15], [101.70, 3.16]];
const [lon, lat, heading] = interpolateAlongPath(path, 0.5);
```

### `createAnimationLoop(options)`

requestAnimationFrame loop with play/pause/speed/seek.

```tsx
import { createAnimationLoop } from "zeb/deckgl";

const loop = createAnimationLoop({
  duration: 30000,      // 30s total
  speed: 1,             // playback speed
  loop: true,           // repeat
  onTick: (progress) => {
    // progress: 0-1
    // update layers here
  },
});

loop.play();
loop.pause();
loop.seek(0.5);        // jump to 50%
loop.setSpeed(2);      // 2x speed
loop.destroy();        // cleanup
```

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
  tooltip={true}
/>
```

### Two-way view state sync

```tsx
const [view, setView] = usePageState("mapView", { longitude: 0, latitude: 20, zoom: 2 });

<DeckMap stateKey="mapView" height="400px" />
<p>Zoom: {view.zoom?.toFixed(1)}</p>
```

### Multiple maps sharing a view

```tsx
const [view] = usePageState("sharedView", { longitude: 0, latitude: 0, zoom: 2 });

<DeckMap id="map-a" stateKey="sharedView" layerKey="layersA" height="300px" />
<DeckMap id="map-b" stateKey="sharedView" layerKey="layersB" height="300px" />
```

### Animation / Temporal Playback

```tsx
import { useState, useEffect, useRef } from "zeb";
import DeckMap, { createAnimationLoop, interpolateAlongPath } from "zeb/deckgl";

export default function FleetPlayback() {
  const [vehicles, setVehicles] = usePageState("vehicles", []);
  const loopRef = useRef(null);

  useEffect(() => {
    loopRef.current = createAnimationLoop({
      duration: 60000,
      speed: 1,
      loop: true,
      onTick: (t) => {
        const updated = input.routes.map(route => {
          const [lon, lat, heading] = interpolateAlongPath(route.path, t);
          return { ...route, position: [lon, lat], heading };
        });
        window.__rweSetPageState({ vehicles: updated });
      },
    });
    return () => loopRef.current?.destroy();
  }, []);

  return (
    <DeckMap
      id="fleet"
      height="600px"
      initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 12 }}
      layers={[{
        type: "ScatterplotLayer",
        data: vehicles,
        getPosition: "position",
        getFillColor: [0, 200, 100],
        getRadius: 60,
        pickable: true,
      }]}
      tooltip={true}
    />
  );
}
```

---

## Events

### `zeb:deck:ready`

Fires once when the Deck.gl instance finishes mounting.

```tsx
document.getElementById("my-map").addEventListener("zeb:deck:ready", (e) => {
  const { deck, instance, id } = e.detail;

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
import { deckgl } from "zeb/deckgl";
// deckgl.ScatterplotLayer, deckgl.HeatmapLayer, deckgl.GeoJsonLayer, etc.
// deckgl.FlyToInterpolator, deckgl.LinearInterpolator
// deckgl.PathStyleExtension, deckgl.DataFilterExtension, etc.

// Or via window:
window.__zebDeck.deck.ScatterplotLayer
```

### FlyTo transition

```tsx
const inst = window.__zebDeck.get("my-map");
inst.setViewState({
  longitude: 101.7,
  latitude: 3.1,
  zoom: 14,
  transitionDuration: 1000,
  transitionInterpolator: new (window.__zebDeck.deck.FlyToInterpolator)(),
});
```

---

## Bundle details

| Property | Value |
|----------|-------|
| Packages | `@deck.gl/core` + `layers` + `aggregation-layers` + `geo-layers` + `mesh-layers` + `extensions` |
| Bundle | `runtime/deckgl.bundle.mjs` |
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
