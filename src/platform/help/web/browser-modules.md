# Browser modules: a folder of real browser modules

A browser module folder is a project folder whose files are ordinary ES
modules that run only in the browser. Use one for large client code that keeps
growing: a three.js game, an editor, a simulation. Pages, forms and tables stay
normal page code.

Normal page code is pasted into one script per page and also runs on the
server (`help("web/custom-scripts")`). A browser module folder is the other
world:

| | Normal page code | A browser module folder |
|---|---|---|
| Runs | On the server, then in the browser | Only in the browser |
| How files join | Pasted into each page's one script | Each entry becomes its own script, cached and shared by every page |
| Names | An exported name is unique per page | Every file keeps its own names |
| Imports | `@/…` and `zeb/*` | `./x`, `../y`, `@/…` inside module folders, `zeb/*`, project library entries |
| Work when a file loads | Not allowed (the server runs it) | Allowed: caches, shared geometry, tables |

## Declare a folder

The folder and its entries are part of the project's RWE settings
(`rwe.browser_modules` in `zebflow.yaml`). `zebflow.yaml` is written by
Zebflow, so set them through the settings API or Studio:

```text
PUT /api/projects/{owner}/{project}/settings/rwe
{ "commit_message": "Declare the game", "data": { "browser_modules": [
  { "path": "modules/game", "entries": ["main.ts", "tools.ts", "islands/*.ts"] }
] } }
```

The section is replaced as a whole, so send its other fields too
(`allow_list`, `strict_mode`); leaving `browser_modules` out keeps the folders
as they are.

- `path` is relative to the project's source folder. A file belongs to at most
  one folder; folders may not nest.
- `entries` are patterns relative to the folder: `*` matches inside one
  folder, `**` across folders.
- Changes apply on the next page load. Nothing restarts.

In Studio, a module folder is tinted and marked **module** in the file tree,
its entries are marked **entry** (or **entry · lazy**), and a file's `⋯` menu
offers **Make browser module entry** and **Remove browser module entry**.

## Entries are the doors

Each entry is built, with every file it imports, into one script. Code that two
entries import goes into one shared script, stored once. Files that are not
entries are internal: a page may import entries only, and importing an internal
file is refused with `RWE_IMPORT_MODULE_INTERNAL`, naming the entries to use.

```tsx
// pages/game.tsx: a normal page
import { useEffect, useRef } from "zeb/react";
import { startGame } from "@/modules/game/main";

export default function Game() {
  const canvas = useRef(null);
  useEffect(() => {
    const game = startGame(canvas.current);
    return () => game.stop();
  }, []);
  return <canvas ref={canvas} />;
}
```

On the server the import is an inert stand-in, so the page renders its frame
there; call an entry from effects and event handlers. In the browser the
page loads the entry's script before it runs.

## What a module can do

```ts
// modules/game/objects.ts
const all = import.meta.glob("./objects/*.ts", { eager: true });  // every file of the folder
export function make(kind: string) {
  return (all[`./objects/${kind}.ts`] as any).build();
}
```

- **Names per file.** A hundred files may each export `build`; names two files
  of one script declare are renamed in both (`build$1`, `build$2`).
- **`import.meta.glob`** with fixed patterns, as in Vite: `{ eager: true }`
  bundles every match; without it, each match must be an entry and is loaded
  on demand. `import: "default"` takes one export instead of the module.
- **Loading on demand:** `import("./islands/north.ts")` with a fixed string. The
  target must be an entry, so it is a script of its own.
- **Data and text:** `import zones from "./data/zones.json"`, and
  `import water from "./shaders/water.glsl?raw"` for a file's text. Shader
  files (`.glsl`, `.vert`, `.frag`, `.wgsl`) are accepted project files.
- **Workers:** `new Worker(new URL("./terrain.worker.ts", import.meta.url), { type: "module" })`.
  The worker file must be an entry; it may import the folder's files and `zeb/*`.
- **three.js and other libraries:** `import { Mesh } from "zeb/threejs"`. The
  page and its modules load the same library file, so they share one instance:
  a `Scene` made in a module is a `Scene` to the page.

## Addresses and caching

Every script is named by the SHA-256 of its content and served where the
page's own scripts are, `…/_rwe/scripts/<hash>`, with
`Cache-Control: public, max-age=31536000, immutable`. Scripts import each
other as `./<hash>` and libraries as `../lib/…`, so the same bytes work on
every host. A changed file gets a new hash; scripts that did not change keep
their address and stay cached.

## What is refused

| Code | When | Instead |
|---|---|---|
| `RWE_IMPORT_MODULE_INTERNAL` | page code imports a non-entry file of a module folder | import an entry, or make the file one |
| `RWE_MODULE_BUILD` | a module imports a file outside every module folder, or a bare npm name | move the file into a module folder; use `zeb/*` or a project library |
| `RWE_MODULE_BUILD` | `import()` or `new Worker(new URL(…))` of a file that is not an entry | add it to the entries |
| `RWE_MODULE_BUILD` | a computed `import(name)` | a fixed string, or a lazy `import.meta.glob` |
| `RWE_MODULE_BUILD` | a lazily loaded file imports the file that loads it | split the shared part into its own file |
| `RWE_MODULE_BUILD` | `"./shader.glsl"` without `?raw` | `"./shader.glsl?raw"` |
| `RWE_MODULE_FOLDER` | the folder does not exist, or lists no entries | fix the setting |

## For agents

File listings say what each file is to its folder. The repository API and the
JSON format of `file_list` carry a `module` field:

```json
{ "rel_path": "modules/game/objects/tree.ts",
  "module": { "role": "internal", "folder": "modules/game", "bundled_into": ["modules/game/objects.ts"] } }
```

`role` is `folder` (with its `entries`), `entry` (with `lazy`), or `internal`
(with `bundled_into` once the folder has been built). The text formats of
`file_list` add a short note: `· browser module entry`.
