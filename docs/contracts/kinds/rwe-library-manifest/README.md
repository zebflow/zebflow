# RweLibraryManifest

Status: **review** — spec settled 2026-08-27, code caught up 2026-08-27.

One RWE library — and this kind governs RWE libraries ONLY: an opaque, pre-built runtime bundle plus typed wrappers,
loaded at runtime, never compiled by the RWE compiler. If the compiler
compiles it, it is source (`template_bundle`), not a library.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `RweLibraryManifest` |
| Format | JSON, canonical envelope |
| Document | `manifest.json` beside the library's version directories |
| Lives | `blessed/rwe-libraries/{name}/` (build source) · inside a `rwe_library` hub package · installed at `data/hub/rwe-libraries/{package_id}/` (e.g. `zebflow.deckgl`) |
| Adapter | `src/contracts/kinds/rwe_library_manifest.rs` |

## Spec

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "RweLibraryManifest",
  "metadata": { "name": "zeb/prosemirror" },
  "spec": {
    "name": "zeb/prosemirror",
    "description": "…",
    "exports": ["mountProseEditor", "prosemirror", "ProseEditor"],
    "versions": {
      "full-1.41": {
        "entry": "0.1/runtime/prosemirror.bundle.mjs",
        "source": "offline",
        "package_version": "1.41.x",
        "size_bytes": 243123,
        "integrity": "sha256:…",
        "note": "build provenance, free text"
      }
    }
  }
}
```

## Rules

| Field | Rule |
| --- | --- |
| `metadata.name` | equals `spec.name`, non-empty |
| `exports` | the symbols a page may import; non-empty; the compiler and UI resolve imports against this list and nothing else |
| `versions` | at least one; keys non-empty |
| `entry` | relative path to the runtime bundle inside the library directory; may only descend |
| `source` | packaging: `offline` — the version's bundle bytes are carried inside the package; `hub` — fetched from a hub at install. **`online` — a bare download URL with no digest and no review — is removed**; "host it yourself" is the static repository channel, which is digest-pinned and reviewed. |
| `integrity` | `sha256:` + 64 hex over the entry bundle. **Required — empty refused.** Filled by the publisher/seeder, verified at install and by the dependency report. |
| `size_bytes` | decoded size of the entry bundle; required, non-zero |

Manifest `source` and lock `source` are different facts and both are kept:
the manifest's is **packaging** — whether the version's bytes are carried in
the package or fetched at install; the lock's is **provenance** of the package
(`hub.*` | `direct.*`). Every installed library loads from its installed copy.
They were coherent by accident before this line existed.

## Project libraries (amendment 2026-11-19, candidate)

A library a project built itself from a `LibraryRecipe`
(`library-recipe/README.md`). It is private to the one project it is uploaded
to, and is imported without the `zeb/` prefix: `site-a-lib/world`.

```json
"versions": {
  "0.1.0": {
    "entry": "0.1.0/runtime/all.mjs",
    "size_bytes": 1210,
    "integrity": "sha256:…",
    "source": "offline",
    "package_version": "0.1.0",
    "recipe": "sha256:…",
    "entries": {
      "world": { "file": "0.1.0/runtime/world.mjs", "exports": ["World", "useWorld"] },
      "three/addons/controls/OrbitControls.js": { "file": "0.1.0/runtime/three/addons/controls/OrbitControls.mjs", "exports": ["OrbitControls"] },
      "all":   { "file": "0.1.0/runtime/all.mjs",   "exports": ["World", "useWorld", "OrbitControls"] }
    },
    "workers": { "geometry": "0.1.0/runtime/geometry.worker.mjs" },
    "files": {
      "0.1.0/runtime/all.mjs":             { "size_bytes": 1210,   "integrity": "sha256:…" },
      "0.1.0/runtime/world.mjs":           { "size_bytes": 21430,  "integrity": "sha256:…" },
      "0.1.0/runtime/three/addons/controls/OrbitControls.mjs": { "size_bytes": 512, "integrity": "sha256:…" },
      "0.1.0/runtime/chunk-3f2a9c.mjs":    { "size_bytes": 265343, "integrity": "sha256:…" },
      "0.1.0/runtime/geometry.worker.mjs": { "size_bytes": 193,    "integrity": "sha256:…" },
      "0.1.0/data/terrain.bin":            { "size_bytes": 812000, "integrity": "sha256:…" }
    }
  }
}
```

| Field | Rule |
| --- | --- |
| `entries` | each key is an import subpath (an npm-style path: letters, digits, `-`, `_`, `.`, `/`, no `..`), so `three` and `three/addons/controls/OrbitControls.js` keep their own names; `file` is listed in `files`; `exports` non-empty. `spec.exports` is the union. |
| `entry`, `size_bytes`, `integrity` | still required: the main entry (the `all` bundle, else the first entry), equal to that file's record in `files` |
| `entries.*.types` | optional: the entry's `.d.ts`, a file listed in `files`; for the editor and a type check, never loaded by a page |
| `workers` | name -> a file listed in `files`; served beside the entries, so `new URL("./<name>.worker.mjs", import.meta.url)` finds it |
| `files` | **every** file of the version: entries, chunks, workers, styles, data. Each with `size_bytes` and `integrity`. A file not listed is not served. |
| `recipe` | digest of the recipe folder and `bun.lock` that built it; shown in Studio, never trusted for anything |
| name | `metadata.name` and `spec.name` follow the recipe's name rule; never `zeb/…` |

**Upload.** Studio, the project's Hub page → Upload library takes one
`.tar.gz`: `manifest.json` plus the version folder, regular files only (no
links, no `..`, no absolute paths). Zebflow refuses the whole archive
unless every file is listed, every listed file is present, and every digest and
size match; then it installs to the project's `data/hub/rwe-libraries/`,
locks it in `zeb.lock` (source `direct.file`), and enables it. A name already
installed in the project is replaced only by a higher version, or by the same
version with the same digests. Maintainers only.

**Serving.** Every listed file is served from the page's own host, under the
project's library path, with its content type (`.mjs` as JavaScript). Each
file is checked against its own digest before it is served, not only the entry.

**On the page.** `import { World } from "site-a-lib/world"` lowers to
`globalThis.__zebLibs["site-a-lib/world"]["World"]`, the same as a `zeb/*`
library; the page loads only the entries it imports. On the server each entry
is an inert stand-in (callable, constructible, `undefined` for `then`), so a
library that only runs in effects renders; its real code runs in the browser.

## Overrides (amendment 2026-11-25)

A project may replace a built-in `zeb/*` bundle library in its own pages with
an entry of a library uploaded to it: `spec.rwe.library_overrides` in
`ProjectConfiguration`, `zeb/threejs` → `{ library, entry }`. Pages keep
importing `zeb/threejs`; the browser loads the entry instead. The server keeps
its own stand-in for the built-in library.

- **Replaceable:** the built-in libraries the browser loads as one bundle
  (`zeb/threejs`, `zeb/d3`, `zeb/potoru`, …). Not `zeb/react`, not `zeb/ui/*`.
- **Interface:** the entry exports every name the built-in bundle exports, or
  the override is refused naming the missing ones. Both are read without
  running them: the module's own exports and every `export * from "./…"` it
  passes on inside its package. New names are allowed. An entry that passes on
  names from outside its package cannot be read and is refused.
- **Pages:** every page and component that imports the library is checked
  with the override in place; one that no longer checks refuses the override
  and leaves the setting as it was.
- **Upload:** `…/rwe/libraries/upload?override=zeb/threejs&entry=three` checks
  the names from the archive before anything is installed. A later upload of a
  library that stands in for a built-in one must keep its entry and every
  name, or it is refused.
- **Removal:** removing the override, or the library, loads the built-in
  library again. What a name does when it runs is the uploader's to keep; the
  check covers names only.

## Rejections

Unknown root fields, unknown spec fields, wrong kind, future `apiVersion`,
empty `exports`, empty or path-escaping `entry`, unknown `source`, missing or
malformed `integrity`, zero `size_bytes`.

## Versioning

Adding an optional spec field before first release follows the dated-amendment
rule (`project-configuration/README.md`). After release: new `apiVersion` plus
converter, per `versioning.md`.

## Code catch-up — closed 2026-08-27

Everything the Open section owed is done:

- The validator enforces this spec: `source` ∈ `offline | hub` (`online`
  refused, and no serving path branches on it any more), `integrity` required
  as `sha256:` + 64 hex with empty refused, `size_bytes` required non-zero,
  `entry` may only descend, `exports` non-empty. Unknown fields were already
  refused at every level.
- The twelve blessed manifests carry the real digest and decoded size of their
  entry bundles, written in the canonical form the one writer emits. The values
  are checked in; two `library.rs` tests keep them honest by recomputing the
  digest and size from the bytes the binary embeds and by re-encoding every
  embedded manifest byte-for-byte
  (`every_embedded_manifest_declares_the_digest_and_size_of_its_real_bytes`,
  `every_embedded_manifest_is_canonical_byte_for_byte`).
- Golden fixture at
  `tests/fixtures/contracts/rwe-library-manifest/v1-complete.json`, with a
  byte-for-byte roundtrip test and negative tests covering malformed bytes,
  unknown root/spec/version fields, wrong kind, the pre-rename kind name,
  future `apiVersion`, missing/empty/malformed integrity, zero size, escaping
  entries, and `online` specifically.
- The changed manifest bytes are a new blessed release: the twelve packages
  whose manifests changed reseed as `zebflow.{name}@0.1.1` beside the old
  `0.1.0`, because a coordinate already present is skipped, never rewritten.

## Internal runtime

Zeb React is embedded at `zeb/react/0.1/runtime/` and loaded automatically by
RWE. It is not an installable `rwe_library` Hub package: it has no
`package.yaml` and no user-selectable manifest. This replaces the former
manifest-less `zebflow.preact` seed. Previously seeded packages are not deleted
from existing stores by this source migration.
