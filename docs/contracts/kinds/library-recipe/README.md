# LibraryRecipe

Status: **candidate**, drawn 2026-11-19.

How a project builds its own private RWE library from npm packages and its own
TypeScript. The recipe is source: it lives in the project's repository, and
the same recipe and lockfile always rebuild the same bytes. The build runs
outside Zebflow, in `zeb-bundler`; Zebflow only installs what it produced
(`rwe-library-manifest/README.md` § Project libraries).

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `LibraryRecipe` |
| Format | YAML, canonical envelope |
| Document | `library.yaml`, in any folder of `repo/` (by habit `libraries/<name>/`) |
| Beside it | `bun.lock` (machine-written, exact versions and digests of every package), the sources it names |
| Reader | `zeb-bundler` (build); Zebflow reads only the digest of this folder, recorded in the built library |

## Spec

```yaml
apiVersion: zebflow.com/v1
kind: LibraryRecipe
metadata:
  name: site-a-lib            # pages import site-a-lib/<entry>
spec:
  version: 0.1.0
  entries:                    # subpath -> a source file, or npm:<module> re-exported whole
    world: src/world.ts
    orbit: npm:three/addons/controls/OrbitControls.js
  bundle: all                 # optional: also site-a-lib/all, re-exporting every entry
  all_prefer: {}              # name -> entry, when two entries export one name
  workers:                    # name -> source file, built as <name>.worker.mjs
    geometry: src/workers/geometry.ts
  assets: [data/]             # folders copied byte for byte beside the built files
  define: {}                  # build-time constants, NAME -> JSON value
  packages:                   # npm packages, exact versions only
    three: 0.187.0
  provided:                   # imports taken from Zebflow at run time, never bundled
    zeb/react: zeb/react
  budget_kb: 3000             # total size of every built file
```

## Rules

| Field | Rule |
| --- | --- |
| `metadata.name` | `[a-z][a-z0-9-]*`, at most 40; never `zeb` or `zeb-*`, `npm`, `node`, `http`, `https`, `data`, `blob`, `file` |
| `version` | `x.y.z`, numbers only |
| `entries` / `workers` keys | `[a-z][a-z0-9-]*`; at least one entry; `all` is taken when `bundle: all` |
| source paths | relative, inside the recipe folder after links are resolved; no `..`, no leading `/` |
| `npm:` entries | the module must come from a package in `packages` |
| `packages` | an exact version (`1.2.3`, `1.2.3-rc.1`); never a range, tag, git, URL, `file:`, `link:` or `workspace:` |
| `bun.lock` | required when `packages` is not empty, and must match `packages` |
| `provided` | `zeb/react` always; others name a Zebflow library installed in the project |
| `define` | keys `[A-Z_][A-Z0-9_]*`; values JSON |
| `budget_kb` | over it, the build fails |

Unknown fields at any level are refused.

## What a build must do

1. Install from `bun.lock` with every package's install scripts switched off.
2. Bundle every entry in one pass, so shared code (one copy of `three`) exists once; each worker on its own, as one file.
3. Refuse an import that leaves the recipe folder and the installed packages.
4. Refuse an output file that names the build machine's home, temporary or recipe path.
5. List every bundled package's licence in `THIRD-PARTY-NOTICES.txt`. An unknown licence is a warning: the library is private.
6. Write the output as an `RweLibraryManifest` package (`rwe-library-manifest/README.md` § Project libraries) whose `recipe` field is the digest of the recipe folder and `bun.lock`.
7. Same recipe, same lockfile, same `zeb-bundler` and Bun version: byte-identical output.

## Implementation

`zeb-bundler`, a Bun command kept outside this repository until it is published; Zebflow ships nothing of it. The Zebflow side (upload, serving, the page import) is not implemented yet.
