# RweRweLibraryManifest

Status: **review** — spec settled 2026-08-27, code catching up (see Open).

One RWE library — and this kind governs RWE libraries ONLY: an opaque, pre-built runtime bundle plus typed wrappers,
loaded at runtime, never compiled by the RWE compiler. If the compiler
compiles it, it is source (`template_bundle`), not a library.

## Identity

| | |
| --- | --- |
| API version / kind | `zebflow.com/v1` `RweRweLibraryManifest` |
| Format | JSON, canonical envelope |
| Document | `manifest.json` beside the library's version directories |
| Lives | `blessed/rwe-libraries/{name}/` (build source) · inside a `rwe_library` hub package · installed at `data/hub/rwe-libraries/{name}/` |
| Adapter | `src/contracts/kinds/rwe_library_manifest.rs` |

## Spec

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "RweRweLibraryManifest",
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
| `source` | `offline` (bytes embedded in the binary / carried in the package) or `hub` (installed from a hub). **`online` — a bare download URL with no digest and no review — is removed**; "host it yourself" is the static repository channel, which is digest-pinned and reviewed. |
| `integrity` | `sha256:` + 64 hex over the entry bundle. **Required — empty refused.** Filled by the publisher/seeder, verified at install and by the dependency report. |
| `size_bytes` | decoded size of the entry bundle; required, non-zero |

Manifest `source` and lock `source` are different facts and both are kept:
the manifest's says how the entry **loads** (embedded bytes vs installed copy);
the lock's says where the **package came from** (`embedded` | `hub`). They were
coherent by accident before this line existed.

## Rejections

Unknown root fields, unknown spec fields, wrong kind, future `apiVersion`,
empty `exports`, empty or path-escaping `entry`, unknown `source`, missing or
malformed `integrity`, zero `size_bytes`.

## Versioning

Adding an optional spec field before first release follows the dated-amendment
rule (`project-configuration/README.md`). After release: new `apiVersion` plus
converter, per `versioning.md`.

## Open

- Code catch-up to this spec: validator still accepts `online` and empty
  `integrity`/`size_bytes`; the blessed manifests carry empty integrity and
  some zero sizes; golden fixture and negative tests do not exist yet.
- Canonical serialization (the current machine-written documents carry
  non-canonical indentation).
