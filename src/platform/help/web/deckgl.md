# DeckGL — Geospatial & Large-Scale Data Visualization

WebGL-accelerated map visualization for geospatial data, fleet tracking, simulations, and large datasets.
Fully offline — `@deck.gl/core`, `@deck.gl/layers`, `@deck.gl/aggregation-layers`, `@deck.gl/geo-layers`, `@deck.gl/mesh-layers`, and `@deck.gl/extensions` bundled inline.

---

## Import

```tsx
import DeckMap from "zeb/deckgl";
```

Named exports:

```tsx
import {
  deckgl,                // full deck.gl namespace (all layer classes, views, etc.)
  buildLayer,            // JSON config → Layer instance
  buildLayers,           // array of configs → array of Layer instances
  mountDeckMap,          // imperative mount (host, options) → runtime
  createDeckMapRuntime,  // alias for mountDeckMap
  ensureDeck,            // returns deck namespace object
  // Utilities
  haversine,             // distance between two [lon, lat] points (meters)
  bearing,               // bearing from point A to B (degrees)
  colorRamp,             // interpolate color stops at position t
  interpolateAlongPath,  // [lon, lat, heading] at progress along a path
  createAnimationLoop,   // requestAnimationFrame loop with play/pause/speed
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
  layers={[{
    type: "ScatterplotLayer",
    data: points,
    getPosition: "position",
    getFillColor: [0, 180, 255],
    getRadius: 80,
    pickable: true,
  }]}
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
| `tooltip` | `boolean` | `false` | Enable default tooltip on hover (shows object properties) |
| `background` | `string` | `"transparent"` | Canvas clear colour |

---

## Layer Types

### Basic Layers (`@deck.gl/layers`)

| Type | Use Case |
|------|----------|
| `ScatterplotLayer` | Points on a map — locations, sensors, vehicles |
| `IconLayer` | Custom marker icons |
| `TextLayer` | Text labels |
| `LineLayer` | Straight lines between two points |
| `ArcLayer` | Curved arcs between origin/destination pairs |
| `PathLayer` | Routes, trails, GPS tracks |
| `PolygonLayer` | Filled regions with optional extrusion |
| `SolidPolygonLayer` | High-performance filled polygons |
| `ColumnLayer` | 3D columns rising from the map |
| `GridCellLayer` | Rectangular grid cells |
| `PointCloudLayer` | 3D point clouds |
| `BitmapLayer` | Raster image overlays |

### Aggregation Layers (`@deck.gl/aggregation-layers`)

| Type | Use Case |
|------|----------|
| `HeatmapLayer` | Density heatmaps — risk zones, population density |
| `HexagonLayer` | Hexagonal binning — aggregate points into hex cells |
| `GridLayer` | Rectangular grid aggregation |
| `ContourLayer` | Iso-lines / contour bands |
| `ScreenGridLayer` | Screen-space grid (fast, GPU-aggregated) |

### Geo Layers (`@deck.gl/geo-layers`)

| Type | Use Case |
|------|----------|
| `GeoJsonLayer` | GeoJSON features — the swiss army knife for geo data |
| `TileLayer` | Raster/vector map tiles (OSM, Mapbox, custom) |
| `MVTLayer` | Mapbox Vector Tiles |

### Mesh Layers (`@deck.gl/mesh-layers`)

| Type | Use Case |
|------|----------|
| `ScenegraphLayer` | 3D models (glTF/GLB) positioned on map — vehicles, buildings |
| `SimpleMeshLayer` | Simple 3D geometry overlays |

---

## Declarative Layers

Pass layer config objects with a `type` string. Accessor props accept either
direct values or simple string keys.

```tsx
layers={[
  {
    type: "ScatterplotLayer",
    id: "stops",
    data: busStops,
    getPosition: "coordinates",    // d => d.coordinates
    getFillColor: [0, 180, 255],
    getRadius: 50,
    pickable: true,
  },
  {
    type: "PathLayer",
    id: "routes",
    data: routeLines,
    getPath: "path",               // d => d.path
    getColor: [255, 100, 0],
    getWidth: 3,
    widthMinPixels: 2,
  },
  {
    type: "HeatmapLayer",
    id: "density",
    data: incidents,
    getPosition: "location",
    getWeight: "severity",
    radiusPixels: 40,
    intensity: 1.5,
    threshold: 0.1,
  },
]}
```

### String Accessor Syntax

| Spec | Resolves to |
|------|-------------|
| `"position"` | `d => d.position` |
| `"[lon, lat]"` | `d => [d.lon, d.lat]` |
| `"[0]"` | `d => d[0]` |

For complex accessors (computed values, conditionals), use imperative access.

### Extensions

Pass `extensions` array in layer config for advanced effects:

```tsx
{
  type: "PathLayer",
  data: routes,
  getPath: "coordinates",
  getColor: [0, 200, 255],
  getDashArray: [8, 4],
  extensions: [{ type: "PathStyleExtension", dash: true }],
}
```

Available extensions: `PathStyleExtension`, `DataFilterExtension`, `BrushingExtension`, `CollisionFilterExtension`.

---

## Patterns

### 1. API-First: Data Loads After Fetch

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

When `setPoints(data)` fires `rwe:state:change`, DeckMap's listener rebuilds
the layers from the new data automatically.

### 2. Two-Way View State Sync

```tsx
const [view, setView] = usePageState("mapView", {
  longitude: 0, latitude: 20, zoom: 2,
});

<DeckMap stateKey="mapView" height="400px" />
<p>Zoom: {view.zoom?.toFixed(1)}</p>
```

User pan/zoom updates `view`. Calling `setView(...)` from elsewhere flies the map.

### 3. GeoJSON Routes + Stops

```tsx
const routes = input.routes;  // GeoJSON FeatureCollection
const stops = input.stops;    // [{ name, coordinates: [lon, lat] }]

<DeckMap
  height="600px"
  initialViewState={{ longitude: 101.68, latitude: 3.14, zoom: 12 }}
  layers={[
    {
      type: "GeoJsonLayer",
      id: "routes",
      data: routes,
      getLineColor: [0, 150, 255],
      getLineWidth: 3,
      lineWidthMinPixels: 2,
    },
    {
      type: "ScatterplotLayer",
      id: "stops",
      data: stops,
      getPosition: "coordinates",
      getFillColor: [255, 200, 0],
      getRadius: 40,
      pickable: true,
    },
  ]}
  tooltip={true}
/>
```

### 4. Heatmap Overlay

```tsx
<DeckMap
  height="500px"
  initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 11 }}
  layers={[
    {
      type: "HeatmapLayer",
      id: "risk-zones",
      data: incidents,
      getPosition: "location",
      getWeight: "severity",
      radiusPixels: 60,
      intensity: 2,
      threshold: 0.05,
      colorRange: [
        [255, 255, 178], [254, 204, 92], [253, 141, 60],
        [240, 59, 32], [189, 0, 38],
      ],
    },
  ]}
/>
```

### 5. OSM Tile Basemap (No Mapbox Required)

```tsx
<DeckMap
  height="100vh"
  initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 13 }}
  layers={[
    {
      type: "TileLayer",
      id: "osm-tiles",
      data: "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
      minZoom: 0,
      maxZoom: 19,
      tileSize: 256,
      renderSubLayers: "bitmap",
    },
    {
      type: "ScatterplotLayer",
      id: "points",
      data: myPoints,
      getPosition: "position",
      getFillColor: [255, 0, 0],
      getRadius: 100,
    },
  ]}
/>
```

The `renderSubLayers: "bitmap"` shorthand tells the runtime to render tiles as `BitmapLayer` sub-layers.

### 6. 3D Columns (Bar Chart on Map)

```tsx
<DeckMap
  height="500px"
  initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 12, pitch: 45 }}
  layers={[
    {
      type: "ColumnLayer",
      id: "trip-counts",
      data: stations,
      getPosition: "coordinates",
      getElevation: "tripCount",
      getFillColor: "color",
      radius: 80,
      elevationScale: 5,
      pickable: true,
    },
  ]}
  tooltip={true}
/>
```

### 7. Animation / Temporal Playback

Use `createAnimationLoop` for smooth vehicle tracking, temporal simulations, and playback:

```tsx
import { useState, useEffect, useRef } from "zeb";
import DeckMap, { createAnimationLoop, interpolateAlongPath } from "zeb/deckgl";

export default function FleetPlayback() {
  const [vehicles, setVehicles] = usePageState("vehicles", []);
  const [progress, setProgress] = usePageState("progress", 0);
  const loopRef = useRef(null);

  useEffect(() => {
    loopRef.current = createAnimationLoop({
      duration: 60000,    // 60 second full playback
      speed: 1,
      onTick: (t) => {
        setProgress(t);
        // Update vehicle positions from their routes
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
    <div>
      <DeckMap
        id="fleet"
        height="600px"
        initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 12 }}
        layers={[
          {
            type: "ScatterplotLayer",
            id: "vehicles",
            data: vehicles,
            getPosition: "position",
            getFillColor: [0, 200, 100],
            getRadius: 60,
            pickable: true,
          },
        ]}
        tooltip={true}
      />
      <div className="flex gap-2 mt-2">
        <button onClick={() => loopRef.current?.play()}>Play</button>
        <button onClick={() => loopRef.current?.pause()}>Pause</button>
        <input
          type="range" min="0" max="1" step="0.001"
          value={progress}
          onInput={(e) => loopRef.current?.seek(Number(e.target.value))}
        />
      </div>
    </div>
  );
}
```

### 8. Real-Time WebSocket Tracking

Combine DeckMap with Zebflow's WebSocket pipelines for live fleet tracking:

```tsx
import { useState, useEffect } from "zeb";
import DeckMap from "zeb/deckgl";

export default function LiveFleet() {
  const [vehicles, setVehicles] = usePageState("vehicles", []);

  useEffect(() => {
    const ws = new WebSocket(
      `${location.protocol === "https:" ? "wss:" : "ws:"}//${location.host}/ws/${input.owner}/${input.project}/rooms/fleet`
    );
    ws.onmessage = (e) => {
      const msg = JSON.parse(e.data);
      if (msg.type === "state_patch" || msg.type === "event") {
        setVehicles(prev => {
          const map = new Map(prev.map(v => [v.id, v]));
          for (const v of msg.payload?.vehicles || []) {
            map.set(v.id, { ...map.get(v.id), ...v });
          }
          return [...map.values()];
        });
      }
    };
    return () => ws.close();
  }, []);

  return (
    <DeckMap
      id="live-fleet"
      height="100vh"
      initialViewState={{ longitude: 101.7, latitude: 3.1, zoom: 12 }}
      layers={[
        {
          type: "ScatterplotLayer",
          id: "vehicles",
          data: vehicles,
          getPosition: "position",
          getFillColor: [0, 200, 100],
          getRadius: 40,
          pickable: true,
        },
      ]}
      tooltip={true}
    />
  );
}
```

### 9. Multiple Synchronized Maps

```tsx
const [view] = usePageState("sharedView", { longitude: 0, latitude: 0, zoom: 2 });

<DeckMap id="map-a" stateKey="sharedView" layerKey="layersA" height="300px" />
<DeckMap id="map-b" stateKey="sharedView" layerKey="layersB" height="300px" />
```

Both maps stay in sync as the user pans either one.

---

## Utility Functions

### `haversine(a, b)`

Returns distance in meters between two `[longitude, latitude]` points.

```tsx
import { haversine } from "zeb/deckgl";
const dist = haversine([101.68, 3.14], [101.70, 3.16]); // meters
```

### `bearing(a, b)`

Returns bearing in degrees (0-360) from point A to point B.

```tsx
import { bearing } from "zeb/deckgl";
const deg = bearing([101.68, 3.14], [101.70, 3.16]); // degrees
```

### `colorRamp(t, stops)`

Interpolates a color ramp at position `t` (0-1). Returns `[r, g, b, a]`.

```tsx
import { colorRamp } from "zeb/deckgl";

// Built-in ramps: "green-red", "blue-red", "cool", "warm", "viridis"
const color = colorRamp(0.7, "green-red"); // [r, g, b, 255]

// Custom stops: [[position, [r, g, b]], ...]
const color2 = colorRamp(0.5, [
  [0,   [0, 100, 255]],
  [0.5, [255, 255, 0]],
  [1,   [255, 0, 0]],
]);
```

### `interpolateAlongPath(path, progress)`

Returns `[longitude, latitude, heading]` at a given progress (0-1) along a coordinate array.

```tsx
import { interpolateAlongPath } from "zeb/deckgl";

const path = [[101.68, 3.14], [101.69, 3.15], [101.70, 3.16]];
const [lon, lat, heading] = interpolateAlongPath(path, 0.5);
```

### `createAnimationLoop(options)`

Returns a controller for requestAnimationFrame-based animation.

```tsx
import { createAnimationLoop } from "zeb/deckgl";

const loop = createAnimationLoop({
  duration: 30000,          // total duration in ms
  speed: 1,                 // playback speed multiplier
  loop: true,               // repeat when done
  onTick: (progress) => {   // called each frame, progress 0-1
    // update positions, layers, etc.
  },
});

loop.play();                // start
loop.pause();               // pause
loop.seek(0.5);             // jump to 50%
loop.setSpeed(2);           // 2x speed
loop.destroy();             // cleanup
```

---

## Events

### `zeb:deck:ready`

Fires once when the Deck.gl instance finishes mounting. Use for imperative setup:

```tsx
document.getElementById("my-map").addEventListener("zeb:deck:ready", (e) => {
  const { deck, instance, id } = e.detail;
  // Full imperative access to deck.gl
  instance.setLayers([...]);
});
```

---

## Imperative API — `window.__zebDeck`

```tsx
const inst = window.__zebDeck.get("my-map");  // instance | undefined

inst.deck                // raw Deck.gl instance
inst.setLayers(layers)   // replace layers (Layer instances or JSON configs)
inst.setViewState(vs)    // fly to new view state
inst.destroy()           // finalize + remove from registry
```

### Raw Deck.gl Access

```tsx
import { deckgl } from "zeb/deckgl";

// All layer classes available:
deckgl.ScatterplotLayer
deckgl.HeatmapLayer
deckgl.GeoJsonLayer
deckgl.TileLayer
deckgl.PathStyleExtension
deckgl.FlyToInterpolator
// ... etc.

// Or via window:
window.__zebDeck.deck.ScatterplotLayer
```

### FlyTo Transition

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

## Pipeline Patterns

### Serve a Map Page

```
| trigger.webhook --path /fleet --method GET
| pg.query --credential main-db -- "SELECT id, name, ST_AsGeoJSON(geom)::json as geometry FROM vehicles"
| n.web.response --template pages/fleet-map.tsx
```

### API Endpoint for Map Data

```
| trigger.webhook --path /api/locations --method GET
| pg.query --credential main-db -- "SELECT id, name, longitude, latitude, value FROM locations"
| n.web.json_response
```

### Real-Time Tracking via WebSocket

```
| n.trigger.ws --room fleet --event telemetry
| n.ws.sync_state --op merge --path /vehicles/{payload.id} --value_path /payload
```

### Aggregated Data for Heatmap

```
| trigger.webhook --path /api/incidents --method GET
| pg.query --credential main-db -- "SELECT longitude as lon, latitude as lat, severity FROM incidents WHERE created_at > NOW() - INTERVAL '7 days'"
| n.web.json_response
```

---

## Full Example: Fleet Dashboard

```tsx
import { useState, useEffect, useRef } from "zeb";
import DeckMap, { haversine, colorRamp, createAnimationLoop } from "zeb/deckgl";

export default function FleetDashboard() {
  const [vehicles, setVehicles] = usePageState("vehicles", input.vehicles || []);
  const [selected, setSelected] = useState(null);
  const [view, setView] = usePageState("mapView", {
    longitude: 101.7, latitude: 3.14, zoom: 12,
  });

  const layers = [
    {
      type: "ScatterplotLayer",
      id: "vehicles",
      data: vehicles,
      getPosition: "[longitude, latitude]",
      getFillColor: vehicles.map(v =>
        v.status === "active" ? [0, 200, 100] : [150, 150, 150]
      ),
      getRadius: 40,
      pickable: true,
      radiusMinPixels: 4,
      radiusMaxPixels: 20,
    },
    {
      type: "PathLayer",
      id: "routes",
      data: input.routes || [],
      getPath: "coordinates",
      getColor: [100, 150, 255, 128],
      getWidth: 3,
      widthMinPixels: 1,
    },
  ];

  return (
    <div className="flex flex-col h-screen">
      <DeckMap
        id="fleet-map"
        height="100%"
        stateKey="mapView"
        layers={layers}
        tooltip={true}
        className="flex-1"
      />
      {selected && (
        <div className="absolute bottom-4 left-4 bg-surface p-4 rounded-lg shadow-lg">
          <h3 className="font-medium">{selected.name}</h3>
          <p className="text-sm text-muted">{selected.status}</p>
        </div>
      )}
    </div>
  );
}
```
