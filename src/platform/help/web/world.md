# zeb/world

```tsx
import { World, useWorld, useElement, useWorldEvent } from "zeb/world";
import { earth, plane, imagery, terrain, layer, data, scale } from "zeb/world";
```

`zeb/world` is [aslnut](https://github.com/aslnut/aslnut), a world engine for the
browser (WebGPU, with WebGL2 where WebGPU is missing): the Earth with imagery and
terrain, or a flat floor, your data drawn over it as layers, and things that stand
and move on it. Apache-2.0, bundled offline; nothing is fetched from a CDN.

Two kinds of names come out of it:

- **The component**, Zebflow's own Zeb React wrapper with exactly aslnut's
  `aslnut/react` interface, built on aslnut's `shell(w)`: `<World>` and the hooks
  `useWorld()`, `useElement(id)` and `useWorldEvent(name, fn)`.
- **aslnut's whole API**, unchanged: `earth`, `plane`, `imagery`, `terrain`, `layer`,
  `data`, `scale`, `theme`, `shape`, `entities`, `legend`, `liveGraphics`, … Its own
  guide is the reference for these (`authoring.md` in the aslnut repository). They make
  plain objects; nothing is drawn until `<World>` mounts.

## A page

```tsx
import { useMemo, useState } from "zeb/react";
import { World, useWorldEvent, earth, imagery, terrain, layer, data, scale } from "zeb/world";

export default function Page(input) {
  const places = useMemo(() => data.geojson(input.places_url, { id: "id" }), [input.places_url]);
  const kind = useMemo(() => scale.category((d) => String(d.props.kind)), []);
  const options = useMemo(() => ({
    body: earth({ imagery: [imagery.sentinel()], terrain: [terrain.mapzen()] }),
    view: { lon: 144.96, lat: -37.81, zoom: 11, tilt: 50 },
  }), []);
  return (
    <World className="h-[70vh] w-full" options={options}
      elements={[layer({ id: "places", data: places, points: { size: 10, color: kind } })]}>
      <Picked />
    </World>
  );
}

function Picked() {
  const [name, setName] = useState("");
  useWorldEvent("tap", (picked) => setName(picked ? String(picked.props.name) : ""));
  return name ? <div className="absolute left-3 top-3 rounded bg-white px-2 py-1 text-sm">{name}</div> : null;
}
```

Every file imports what it uses: `Picked` calls `useWorldEvent`, so a component in
another file imports it from `"zeb/world"` itself.

## `<World>`

| Prop | What it does |
|---|---|
| `options` | What the world is made with (`body`, `view`, `time`, `theme`, `quality`, …). Made once per mount; see below for which ones follow later changes |
| `elements` | aslnut elements (`layer(…)`, `entities(…)`, `roads(…)`, …), each with an `id` |
| `onReady` | `(w) => …`, once per world, after its elements are in. `w` is aslnut's plain world: `w.fly`, `w.on`, `w.api`, `w.list` |
| `viewChange` | `"go"`, `"fly"` or fly options: how a changed `options.view` moves the camera. Unset, the camera stays where the person put it |
| `headless` | No canvas, nothing drawn (`world(null)`): for tests |
| `className`, `style` | The box the world fills. Give it a size (`h-[70vh] w-full`); it is `position: relative` unless `style` says otherwise |
| `children` | Drawn over the world: panels, legends, buttons. They read the world with the hooks |

The server renders the box with an empty canvas slot and nothing else. The world is
made in the browser when the page mounts, on a canvas of its own, and disposed when
it unmounts. `w.api` is `"webgpu"` or `"webgl2"`; `w.reason` says why.

## Rules a page author must know

1. **Elements are a list, and every element has an `id`.** The list is kept in step by
   id: a new id is added, a gone id is removed, a changed prop is set on the element
   already in the world (`setProps`). A restyle is never a rebuild. An element without
   an `id` gets a new one each render, so it is added again each render.
2. **Make sources, scales, themes and functions once** (`useMemo`, or a module-level
   constant). Plain objects and arrays are compared by value, but a source
   (`data.geojson(url)`), a scale (`scale.category(fn)`) or a function made inline in
   render is a new object each time and counts as changed: a restyle at best, a data
   load at worst. The `useMemo` calls in the page above are not decoration.
3. **The camera is the person's.** A changed `options.view` moves nothing unless
   `viewChange` says `"go"` or `"fly"`; without it a re-render never snaps the camera
   back from where the visitor left it.
4. **Options with a setter follow their changes, the rest are the world's for its
   life:** `views`, `time`, `timeZone`, `quality`, `render.shadows`, `theme`, `budget`,
   `priorities` and `observation` follow. `body`, `graphics`, `loader`, `controls` and
   `clock` do not: a different one needs a new world (`key={…}` on `<World>`).
5. **Children of `<World>` are overlays.** They sit over the canvas and read the world
   with `useWorld()` (null until it is ready and after it is gone), `useElement(id)`
   and `useWorldEvent(name, fn)` (`tap`, `hover`, `select`, `elements`, `level`,
   `frame`, `error`, `long`, `governor`). A hook used outside a `<World>` finds no
   world.
6. **Data credits stay on screen.** Every world draws the credits of what it shows in
   a corner of the canvas. Do not hide or crop them (`attribution: false` is only for
   an app that shows `w.attribution()` itself, always).
7. **earth-lite ships with Zebflow:** an offline basemap (Sentinel-2 cloudless 2020 by EOX,
   CC BY 4.0, levels 0 to 4) served at `/assets/libraries/zeb/world/earth-lite/{z}/{x}/{y}.jpg`:
   `earthLite({ url: "/assets/libraries/zeb/world/earth-lite", maxZoom: 4 })` draws a globe with
   no internet at all, as the Studio's own maps do.
8. **Esri and OpenStreetMap tiles are show-only:** drawn while looked at, never
   prefetched, never kept, never exported or recorded for publication. `imagery.sentinel()`
   (EOX, CC BY 4.0) and `terrain.mapzen()` may be kept, with their credits. A basemap you
   host yourself carries your own terms. The full table is `DATA-LICENCES.md` inside the
   package.
9. **Never gate what is shown by the camera.** `when: { zoom }`, `{ dist }`, `{ tilt }`
   throw. A digital twin is looked at through levels (`observation: { levels: "default" }`
   and `when: { level: { from: "district" } }`).

## On the server

The server cannot draw, so `zeb/world` has a stand-in there: `<World>` renders its box,
`useWorld()` and `useElement()` answer `null`, `useWorldEvent()` does nothing, and every
other name (`earth`, `layer`, `data.geojson`, `scale.category`, …) is one inert value you
can call, read and `await` during render. Build options and elements in render as above;
they are only used once the page runs in the browser. Anything that needs the real world
(`w.fly`, `w.pick`, `liveGraphics()`) runs from `onReady`, an event or an effect.

## Tile worker

Street-level tiles and baked roads mesh off the main thread when given a worker lane. The
bundle serves one built with aslnut's defaults:

```tsx
import { worker, tilesWorkerUrl } from "zeb/world";
// in an effect: const lane = worker(tilesWorkerUrl);  streets({ …, worker: lane })
```

It serves `streetsWorker()` and `bakedRoadsWorker()` with their default look. A street
level made with its own `look` needs its own worker module (`serve(streetsWorker(look))`);
without a worker aslnut builds the tiles on the main thread.

## Baked roads

`bakedRoads({ url: ".../index.json" })` reads a road pack: `index.json`, a tile per 500 m,
`clamp.json` (read it with `clampTerrain(terrain, ".../clamp.json")`), `terrain.json` and
`walk.json`. A pipeline makes one on the server with the `world.roads.bake` node
(`help("pipeline/nodes/world.roads.bake")`): it bakes a seed folder from the project's store
with aslnut's Rust road baker and writes the plain `.json` files under `--folder`.

`world.roads.bake --from world/seed --place site-a --folder world/packs/roads/site-a`

The seed is OpenStreetMap road lines and a ground grid (aslnut's `roads/extract.py` and
`roads/dem.py` make them today). The pack is built from OpenStreetMap data (ODbL) and
records its sources and credits in `index.json`; the world draws them, so do not hide them.

## Testing a page

Expose the world from `onReady` (`globalThis.__world = w`) and drive it from a browser
test. `w.api` says which graphics ran, `w.list` holds the elements, `w.on("elements", fn)`
says whether an update was `props` (a restyle) or `add` / `remove`, and `liveGraphics()`
reads zero graphics, buffers, textures and bytes once every world is disposed. Headless
Chromium draws WebGL2 in software: give waits generous timeouts.
