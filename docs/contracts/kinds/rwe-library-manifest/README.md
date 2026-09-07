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
