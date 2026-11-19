# A project's own libraries

A project can carry its own browser library, built from npm packages and its
own TypeScript, private to that project. It is built outside Zebflow by
`zeb-bundler` from a recipe in the repository, uploaded as one `.tar.gz`, and
imported by pages like a `zeb/*` library, under its own name.

```tsx
import { mountScene } from "site-a-lib/scene";                  // the project's own code
import { Scene, Mesh } from "site-a-lib/three";                  // an npm package, under its own name
import { OrbitControls } from "site-a-lib/three/addons/controls/OrbitControls.js";
```

## The recipe (`libraries/site-a-lib/library.yaml`, in the repository)

```yaml
apiVersion: zebflow.com/v1
kind: LibraryRecipe
metadata: { name: site-a-lib }       # pages import site-a-lib/<entry>
spec:
  version: 0.1.0
  entries:
    scene: src/scene.ts              # own code, own name
    three: npm:three                 # an original package, its own name
    three/addons/controls/OrbitControls.js: npm:three/addons/controls/OrbitControls.js
  packages: { three: 0.187.0 }       # exact versions only; bun.lock beside the recipe
  provided: { zeb/react: zeb/react }
  budget_kb: 3000
```

Code inside the library keeps its original imports (`import * as THREE from
"three"`); the bundler joins them and a shared package exists once. Workers are
found where the code starts them, `new Worker(new URL("./x.worker.ts",
import.meta.url), { type: "module" })`, and built as their own files. Folders in
`assets` travel as data files. The full rules: `kinds/library-recipe`.

## Build and upload

`zeb-bundler` is not published yet; until it is, whoever maintains the
project builds the archive.

1. `zeb-bundler lock libraries/site-a-lib`, then `zeb-bundler build libraries/site-a-lib --out dist`
   gives `dist/site-a-lib-0.1.0.tar.gz`. The same recipe and lockfile always give the same bytes.
2. Studio → the project's **Hub** → **Upload library**, or
   `POST /api/projects/{owner}/{project}/rwe/libraries/upload` with the file as the body
   (`Content-Type: application/gzip`). Maintainers only.
3. The answer names what pages may import. It is listed by
   `GET /api/projects/{owner}/{project}/rwe/libraries` (`"project_library": true`,
   `imports`) and locked in `zeb.lock` as `direct.file`.

An upload replaces the installed library only with a higher version; the same
version with the same files changes nothing. Remove it with
`DELETE /api/projects/{owner}/{project}/rwe/libraries/remove?name=site-a-lib`.

## What is refused

The whole upload is refused, naming the file and the reason, when:

- the archive holds anything but plain files on plain paths (no links, `..`,
  absolute paths, duplicates), or is over 64 MiB packed, 256 MiB unpacked, 4000 files;
- a file is not listed in `manifest.json` with its size and SHA-256, or does not match;
- code uses `eval` or `new Function`, loads code from another site
  (`import("https://…")`, `importScripts`), or names a path of the machine that built it;
- a file is a program, a page (`.html`), a source map, a scripted SVG, or not
  WebAssembly while named `.wasm`.

Every file is checked against its fingerprint again before it is served; a file
changed after upload answers 409.

## On the page

The page loads only the entries it imports, from the page's own host, so a
library's workers and data files load beside it (`new URL(…, import.meta.url)`).
On the server each entry is an inert stand-in: call library code from
`useEffect` or event handlers, never during render. An entry the library does
not have is refused when the page compiles, naming the entries it has. A
library's types (`.d.ts`) travel in it for the editor; they are not loaded by pages.
