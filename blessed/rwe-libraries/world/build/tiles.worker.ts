// The tile worker of zeb/world: aslnut's street-level and baked-road meshing, with aslnut's own defaults.
//   import { worker, tilesWorkerUrl } from "zeb/world";
//   const lane = worker(tilesWorkerUrl);       // pass it as `worker: lane` to streets(…) / bakedRoads(…)
// A street level made with its own `look` needs a worker of its own that serves streetsWorker(look).
import { serve } from "@aslnut-src/core/runtime/workers.ts";
import { streetsWorker } from "@aslnut-src/modules/world/earth/street.ts";
import { bakedRoadsWorker } from "@aslnut-src/modules/world/earth/presets/built/roads/baked.ts";

serve({ ...streetsWorker(), ...bakedRoadsWorker() });
