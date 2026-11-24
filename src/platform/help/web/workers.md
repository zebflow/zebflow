# Workers: project code off the page's thread

Heavy work (a simulation, a model over a large array, parsing a big file)
belongs in a worker, so the page stays smooth. A file whose name ends in
`.worker.ts` is a worker; importing it gives its address:

```tsx
import { useEffect, useState } from "zeb/react";
import heatWorker from "@/modules/heat/heat.worker.ts";

export default function HeatPanel() {
  const [cells, setCells] = useState<number[]>([]);
  useEffect(() => {
    const worker = new Worker(heatWorker, { type: "module" });
    worker.onmessage = (event) => setCells(event.data);
    worker.postMessage({ width: 64, height: 64 });
    return () => worker.terminate();
  }, []);
  return <p>{cells.length} cells</p>;
}
```

```ts
// modules/heat/heat.worker.ts
import { step } from "@/modules/heat/model.ts";        // project files are bundled in
import { grid } from "site-a-lib/heat";               // a project library entry, from the same host

self.onmessage = (event: MessageEvent<{ width: number; height: number }>) => {
  self.postMessage(step(grid(event.data.width, event.data.height)));
};
```

- **Live:** a save of the worker or of any file it imports takes effect on the
  next page load. No rebuild, no upload.
- **Imports:** `@/…` project files, bundled into the worker's one script; the
  project's own library entries (`site-a-lib/…`, only in the worker file itself).
  No `zeb/*` libraries and no `zeb/react`: a worker has no page.
- **Only a default import** of a worker; then `new Worker(address, { type: "module" })`.
- **Refused:** `eval`, `new Function`, code from other sites, `importScripts`,
  a worker importing another worker.
- **Talking:** `postMessage` both ways; send large arrays as transferables
  (`postMessage(buffer, [buffer])`) so they move instead of being copied.

Inside a browser module folder a worker is started the standard way,
`new Worker(new URL("./x.worker.ts", import.meta.url), { type: "module" })`,
and may import `zeb/*` libraries: `help("web/browser-modules")`.

A worker is not a service worker: it lives with one page and ends with it.
