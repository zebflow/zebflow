# zeb/world

[aslnut](https://github.com/aslnut/aslnut) as a Zebflow runtime library: a world engine on
WebGPU and WebGL2 (the Earth with imagery and terrain, or a flat floor; your data drawn over
it as layers; things that stand and move on it). Apache-2.0. Fully offline: the engine, its
lazy chunks and the tile worker ship inside the package; nothing is fetched from a CDN.

The page-author guide is `help("web/world")`. This file is for whoever maintains the package.

## Import

```tsx
import { World, useWorld, useElement, useWorldEvent } from "zeb/world";
import { earth, plane, imagery, terrain, layer, data, scale } from "zeb/world";
```

Two kinds of names (`manifest.json` lists every one):

- Zebflow's own Zeb React wrapper (`src/react.ts`, MIT) with exactly aslnut's `aslnut/react`
  interface, built on aslnut's `shell(w)`: `World`, `useWorld`, `useElement`,
  `useWorldEvent`. The wrapper's `World` is the one exported as `World`; the root's `World`
  class stays reachable as the `w` of `onReady` and the value of `useWorld()`;
- aslnut's whole root API (`aslnut`), unchanged: `world`, `earth`, `plane`, `imagery`,
  `terrain`, `layer`, `data`, `scale`, `theme`, `shape`, `entities`, `liveGraphics`, ... and
  `tilesWorkerUrl`, the one name this package adds (see Tile worker).

## How it binds

- **Zeb React, not React.** The wrapper imports eight names from `"zeb/react"`
  (`createContext`, `createElement`, `useCallback`, `useContext`, `useEffect`, `useRef`,
  `useState`, `useSyncExternalStore`). The build aliases `"zeb/react"` to `build/zeb-react.mjs`,
  which reads `globalThis.__zebReact` when the bundle is evaluated; all eight exist in
  `src/rwe/runtime/zeb_react.js`. aslnut's own React shell is not in the bundle.
- **The server has a stand-in** (`src/rwe/runtime/zeb_ssr_libs.js`, key `zeb/world`):
  `World` renders exactly the box the wrapper's `World` renders before its world exists, so hydration matches
  (`<div style="position:relative"><div style="position:absolute;inset:0"></div>…</div>`);
  `useWorld` and `useElement` answer `null`; `useWorldEvent` does nothing; every other name
  is one inert value that can be called, constructed, read and awaited (`then`,
  `Symbol.iterator`, `Symbol.toPrimitive` and `toJSON` are `undefined`).
- **Nothing is a global.** A page's `import { earth } from "zeb/world"` is lowered to
  `globalThis.__zebLibs["zeb/world"]["earth"]` (`src/rwe/core/zeb_lib.rs`).

## Tile worker

`tilesWorkerUrl` is `new URL("./tiles.worker.mjs", import.meta.url).href`.
`0.1/runtime/tiles.worker.mjs` runs `serve({ ...streetsWorker(), ...bakedRoadsWorker() })`
with aslnut's default look, so `worker(tilesWorkerUrl)` is a lane for street-level tiles and
baked roads made with that look. A street level with its own `look` needs a worker module
that serves `streetsWorker(look)`; without any worker aslnut builds on the main thread.

## earth-lite

`earth-lite/` is an offline basemap that ships inside the package and the Zebflow binary:
Sentinel-2 cloudless 2020 by EOX IT Services GmbH (contains modified Copernicus Sentinel data
2020), CC BY 4.0, levels 0 to 4 as 341 tiles of 256 px (about 1.8 MB; EOX answers a tile of one
colour, Antarctica's ice, with a small PNG under the same `.jpg` name). It is served at
`/assets/libraries/zeb/world/earth-lite/{z}/{x}/{y}.jpg` (`image/jpeg`), so a world on an office
with no internet still has a globe:

```tsx
earth({ imagery: [earthLite({ url: "/assets/libraries/zeb/world/earth-lite", maxZoom: 4,
  attribution: "Sentinel-2 cloudless 2020 by EOX IT Services GmbH (contains modified Copernicus Sentinel data 2020), CC BY 4.0" })], terrain: [] })
```

`earth-lite/ATTRIBUTION.txt` is the credit aslnut's tool writes; `earth-lite/LICENCE.txt` the
licence note. The world draws the credit on screen; keep it there.

## Build

```sh
cd <aslnut checkout> && npm ci --ignore-scripts
ASLNUT_SRC=<aslnut checkout> node blessed/rwe-libraries/world/build/build.mjs [--earth-lite <pack>] [--vendor]
```

- `--earth-lite <pack>`: copy a prepared pack into `earth-lite/`, after checking every tile of
  every level is there and is an image. Make the pack once with aslnut's
  `node <aslnut checkout>/packages/aslnut/tools/earth-lite.mjs <pack> 4` (it downloads the 341
  public tiles from EOX). A rebuild without the flag keeps the pack already there.
- `--vendor`: copy the finished package, without `earth-lite/`, into the Studio's pinned copy,
  `src/platform/web/vendor/zeb/world/`, and rewrite its block of `PLATFORM_VENDOR_ASSETS`. The
  Studio's own maps (the database geo preview, the MapServer layer preview) load the library from
  there; earth-lite is data, kept once, and the Studio's lookup finds it in the hub's copy.
- Without `ASLNUT_SRC`, only these two steps run, with the checksums and the embedded list:
  the bundle in `0.1/runtime/` is kept as it is.

aslnut does not depend on esbuild; it builds with Rolldown (through Vite), so the build takes
Rolldown from `ASLNUT_SRC/node_modules` and bundles aslnut's sources directly:

| file | role |
|---|---|
| `src/react.ts` | Zebflow's Zeb React wrapper (`World` and the three hooks) |
| `build/entry.ts` | re-exports aslnut's root, then the wrapper by name, and `tilesWorkerUrl` |
| `build/zeb-react.mjs` | `"zeb/react"` as Zeb React's global |
| `build/interface.mjs`, `build/interface-check.ts` | the interface guard: the wrapper's types must equal `aslnut/react`'s, checked with aslnut's TypeScript before bundling |
| `build/tiles.worker.ts` | the tile worker's entry |
| `build/build.mjs` | the two bundles, notices, `exports.json`, `keywords.json`, `manifest.json`, `FILES-SHA256.txt`, the `zeb/world` block of `PLATFORM_LIBRARY_ASSETS`, the earth-lite copy (`--earth-lite`) and the Studio's pinned copy (`--vendor`) |

The output is one ES module (`--splitting`, minified, es2022, browser) with its lazy chunks
beside it, `node:*` left external (aslnut reaches it only in Node), no source maps. Before it
bundles, the build refuses a wrapper whose interface drifted from `aslnut/react` (it prints
what differs); after, it refuses a bundle that carries aslnut's own React shell, loads the
bundle in Node and refuses it unless the wrapper's `World` is the exported `World`, and refuses
any built file that names a local path.

## Files

| file | |
|---|---|
| `0.1/runtime/world.bundle.mjs`, `*-<hash>.mjs`, `tiles.worker.mjs` | the library (entry, lazy chunks, tile worker); every one must answer next to the entry |
| `0.1/library.json`, `exports.json`, `keywords.json` | editor metadata; `exports.json` is generated |
| `manifest.json`, `package.yaml` | the library contract and the hub seed (`zebflow.world`) |
| `LICENSE`, `NOTICE`, `DATA-LICENCES.md` | aslnut's own, copied unchanged from its package |
| `earth-lite/{z}/{x}/{y}.jpg`, `earth-lite/ATTRIBUTION.txt`, `earth-lite/LICENCE.txt` | the offline basemap (Sentinel-2 cloudless 2020 by EOX, CC BY 4.0) |
| `THIRD-PARTY-NOTICES.txt` | the npm packages inside the bundle, from the module graph |
| `FILES-SHA256.txt` | digest of every file listed above |

## Licence

aslnut is Apache-2.0 (Copyright 2026 aslnut). `LICENSE` and `NOTICE` travel with the code, and
`NOTICE` carries the MIT notices of the code aslnut includes (the UTCI coefficients of
pythermalcomfort and the daylight sky adapted from three.js). aslnut's sources are bundled
unmodified (`modified: false` in the manifest). `DATA-LICENCES.md` is
aslnut's table of the data services its provider factories point at: Esri and OpenStreetMap
tiles are show-only, and a world always draws the credits of what it shows.
