# Project directory

Stability-matrix row 2. Status: **review, with evidence** — applied to the code
and live-verified except where a line below says "migrating".

## The tree

```
<project>/
├── repo/          AUTHORED — yours, git-tracked, portable
├── data/
│   ├── store/     DURABLE — irreplaceable, back this up
│   ├── cache/     DISPOSABLE — delete anytime, rebuilds
│   ├── hub/       INSTALLED — unpacked hub content
│   ├── recovery/  BOUNDED — migration safety copies
│   └── logs/      BOUNDED — invocation records
└── files/         USER DATA — flat namespace, .zebfs/acl.json governs visibility
```

## The rules

| Tier | Rule |
| --- | --- |
| `repo/` | What you authored. Internal layout is **yours**, declared in `zebflow.yaml` (`spec.layout`). The contract prescribes nothing inside it beyond `zebflow.yaml` and `zeb.lock` existing. Machine-generated files never land here. |
| `files/` | Your objects, one flat namespace. Visibility per path via `.zebfs/acl.json` — no public/private directories. |
| `data/store/` | Machine-written, cannot be regenerated. `sekejap/`, `local.db`, `kv.db` *(landed 2026-08-25; a pre-tier global `kv_durable.db` at the data root drains into it per namespace on first touch)*, `chat_history.json` *(landed 2026-08-25; moves from cache/ on first touch)*. |
| `data/cache/` | Machine-written, regenerates. Deleting is never data loss. `pipelines/`, `agent_docs/`. |
| `data/hub/` | Installed from a hub or local document, one child per kind: `nodes/` *(migrating from data/nodes/)*, `rwe-libraries/`. The installed form is the artifact — same contract as any plugin system. Removal is `uninstall`, recorded in `zeb.lock`, never a cleanup. |
| `data/recovery/`, `data/logs/` | Disposable after a retention window. Recovery files are dated; logs hold `invocations.db`. |

One question, one answer: "is this safe to delete?" is answered by the tier,
never by reading source.

## Migration

Tier moves are transparent, on first touch, via `migrate_tier_entry`: one
atomic rename, refuse when both old and new exist. No CLI command — the old
paths have no fallback reader, so a manual-only migration would present every
existing project as empty.

`kv.db` is the one row-level move, because the old `kv_durable.db` holds many
projects' rows in one file. On a namespace's first durable touch
(`MemStateBus`, `src/infra/io/state/mem.rs` — which builds the
`users/{owner}/{project}/data/store/kv.db` path itself, mirroring
`ProjectFileLayout::data_store_kv_db_file`, since infra cannot see platform
types), its rows move in a single cross-database `ATTACH` transaction — a
crash leaves them wholly in one file, never split. A per-project `kv.db`
already holding rows while the global file still has rows for that namespace
refuses, same rule. The global file is never created by current code and is
removed only once it holds zero rows; rows for projects that no longer exist
stay in it untouched — deleting them would destroy their only copy, moving
them would resurrect a removed project's directory, and the file's survival is
the honest record that unclaimed state remains. Project deletion removes the
whole project directory, `kv.db` included.

## Open

- `data/logs/` retention window: unset.
- The data-root level above projects (`platform/`, `services/`, `.bootstrap/`)
  has no tier contract of its own.

## Evidence

Live-verified 2026-08-24/25: fresh projects get all tiers; a seeded old-shape
project migrates with content intact (sekejap byte-identical, sqlite row
queried, not just hashed); second run is a no-op; both-paths-present refuses.
The `data/nodes` cache claim was disproved live — a locally-installed bundle
cannot be rebuilt from `zeb.lock`, which is why the tier is `installed`, not
`cache`. Full history: git log of this file.

`kv.db` and `chat_history.json`, live-verified 2026-08-25 against a scratch
data root seeded with a pre-tier global `kv_durable.db` (three namespaces, one
orphaned) and a cache-tier chat history: each project's rows moved to its own
`data/store/kv.db` on its first `n.kv.* --durable` touch with stored TEXT
byte-exact (`{"hello":"world from legacy"}`, `41` — values queried, not
hashed), the orphan namespace kept the global file alive until an operator
drained it by hand, after which the next touch removed the file; chat history
landed in store SHA-identical with `data/cache/` left empty; second runs were
no-ops; a fresh project wrote durable state straight to its own file with no
global file ever created; and project deletion removed the directory,
`kv.db` included. Both-paths-holding-rows refuses (unit-tested,
`legacy_migration_refuses_when_both_files_hold_rows`).
