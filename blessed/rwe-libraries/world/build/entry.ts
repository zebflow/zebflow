// zeb/world 0.1: aslnut's whole root API, and Zebflow's own Zeb React wrapper (../src/react.ts).
//
// `@aslnut-src/` is aslnut's `packages/aslnut/src` (build.mjs resolves it from ASLNUT_SRC).
// The wrapper's `World` is exported by name below, so it wins over the root's `World` class
// that `export *` would otherwise bring: a page writes <World options elements>, and the
// class stays reachable as `w` in `onReady` and in `useWorld()`.
export * from "@aslnut-src/index.ts";
export { World, useWorld, useElement, useWorldEvent } from "../src/react.ts";

/** The tile worker built beside this file: `worker(tilesWorkerUrl)` runs aslnut's street and road meshing off the main thread. */
export const tilesWorkerUrl: string = new URL("./tiles.worker.mjs", import.meta.url).href;
