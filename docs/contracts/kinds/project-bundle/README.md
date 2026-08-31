# ProjectBundle

Status: **review** — spec settled 2026-08-28; code caught up 2026-08-28
(envelope kind and adapter, transfer-archive alignment with staged
verification and recovery swaps, platform-scope import with store
auto-initiation, fifth report family). Still owed: the hub `project_bundle`
asset channel ships the HubPackage layout rather than this envelope, and a
remote-office import destination stays unbuilt.

This contract defines project export and import archives: one envelope for
every whole-project movement.

## Sealed 2026-08-28 — one envelope, declared classes

One `ProjectBundle` document covers every whole-project movement — transfer
archive, hub `project_bundle` asset, platform-scope install. The manifest
declares which classes the archive carries:

| Class | Content | Full backup | Starter | Content move |
| --- | --- | --- | --- | --- |
| `repo` | `repo/` — source, `zebflow.yaml`, `zeb.lock`, schema/initial-data files | yes | yes | — |
| `store` | `data/store/` — applied database state | yes | — | — |
| `files` | `files/` — objects + `.zebfs/acl.json` | yes | — | yes |

Fixed rules, riding on sealed ground elsewhere:

- `data/cache/` never travels — rebuildable (`instance-directory.md`).
- `data/hub/` never travels raw — it regenerates from the lock; `direct.*`
  lock sources are the exception and the export carries their bytes
  (`distribution.md` §5 reproducibility table).
- `logs/` and `recovery/` stay home.
- `store` without `repo` is refused: state without the source that explains
  it is not a project.

### Schema and initial data

Schema and initial-data files ride inside `repo/` — no class of their own.
The initial-data files (the layout's `initial_data` directories) are authored
state; the structure documents under the layout's `schema` and `sqlite_schema`
directories are written by the platform from the live store.

| Bundle carries | On import, the database is |
| --- | --- |
| `repo` only | auto-initiated: declared initial-data steps replay into a fresh store |
| `repo` + `store` | restored: the store snapshot already contains the applied schema; steps do not replay |

What a schema file *is* — engines, identity, transactions — belongs to
`DatabaseSchema`, which references this rule instead of restating it.

### Import into an existing project — replace per class, staged

Import into an existing project replaces what it carries, keeps what it does
not, and never merges. Merging source is git's job through the git-remote
channel; a database cannot be file-merged at all.

1. Extract to `tmp/transfer/{op}/` (EPHEMERAL staging).
2. Verify before touching the project: manifest identity, checksums, path
   safety.
3. Per carried class, one atomic swap: the current directory moves to
   `recovery/{class}-{date}/` (BOUNDED tier), the bundle's directory moves in.
4. Classes the bundle does not carry are untouched.
5. Rollback is the reverse swap from the recovery copy — a real promise: the
   displaced bytes exist until retention removes them.

### Identity — two imports, one rule each

The archive's `metadata.name` is provenance, never authority. It is recorded
and shown; it gates nothing.

- **Project import** (inside a project): targets that project, always — you
  are here, you chose import, this project is what gets replaced per carried
  class. The wrong archive is survivable, not impossible: the recovery copy is
  the safety, not a name check.
- **Platform import** (dashboard): creates the project; the importer supplies
  owner, project name, and office. The office is the one the import runs on;
  directing an import at a remote office is the same unbuilt wiring as remote
  `hub-public` provisioning (`stability-matrix.md` row 14) and is recorded,
  not designed here. Placement is the operator's choice at import time —
  answering `OfficeTopology`'s open question for this path: the project does
  not express it.

Clone, migrate, rename, and template are all the platform import with a
different destination; none is a mechanism of its own.

### Envelope

`manifest.json` at the archive root is the contract document:

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "ProjectBundle",
  "metadata": { "name": "superadmin/spatial-blog" },
  "spec": {
    "classes": ["repo", "store", "files"],
    "exported_at": 1787175600,
    "source_office_id": "office-a",
    "class_digests": {
      "repo": "sha256:aa11…",
      "store": "sha256:bb22…",
      "files": "sha256:cc33…"
    },
    "carried_dependencies": [
      { "name": "zeb/my-chart", "source": "direct.file", "integrity": "sha256:ab12…" }
    ],
    "counts": { "repo": 41, "store": 7, "files": 128, "total_bytes": 5242880 }
  }
}
```

| Rule | |
| --- | --- |
| `classes` | non-empty subset of `repo`, `store`, `files`; `store` without `repo` refused |
| `class_digests` | one tree digest per carried class, `sha256:` + 64 hex; verified at staging before any swap — the class swaps as a unit, so it verifies as a unit |
| `carried_dependencies` | every `direct.*` lock entry travels with its bytes; `hub.*` entries regenerate from the lock |
| paths | every archive entry descends — no `..`, no absolute paths, no symlink escaping the extract root; refused at staging |
| whole archive | its sha256 lives in the operation record, not the manifest |

### The repo class is whole

There is no partial repo selection: `n.function.call` targets always travel.
Dangling intra-project references are the business of partial carriers
(`folder_bundle`, `template_bundle` — HubPackage). Import verifies function
targets resolve in the carried repo and reports misses through the dependency
report as its fifth family — report, not refuse.

## Recorded before review

### Intra-project references can dangle

Found while reviewing `NodeBundle`, recorded here because it is this contract's
concern rather than that one's.

`n.function.call` invokes another pipeline in the same project by slug, and
`n.trigger.function` is the entry point it targets. Nothing verifies that the
target resolves. A project copied without the called pipeline therefore carries
a reference to something that does not exist, and the failure appears only at
run time.

This is the same class of problem as a copied Python project missing a file.
It is distinct from a missing node implementation:

| Missing thing | Obtainable? | Described by |
| --- | --- | --- |
| node implementation | yes — install the package, or scaffold from its interface | `repo/nodes/{kind}.json` |
| function pipeline | **no** — it should have travelled with the project | nothing today |

A missing node can be repaired. A missing function pipeline cannot be fetched
from anywhere, so the honest response is to report precisely what is absent
rather than to offer a repair that does not exist. ProjectBundle therefore has
to decide which intra-project references an export must carry, and import has to
verify them.

### Detection belongs to one report

`DependencyLockService::status` already reports four families: `rwe_library`,
`node_bundle`, `node_kind`, and `pipeline_source`. Unresolved function calls
should become a fifth family there rather than a separate mechanism, so a
project has one answer to "what is missing".

That report is served by `GET /api/projects/{owner}/{project}/dependencies` but
is not consulted when a pipeline is opened, which is why an author currently
meets a broken node before meeting the explanation. Surfacing it on open is a UI
decision, recorded here so it is not rediscovered.
