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
| `data/store/` | Machine-written, cannot be regenerated. `sekejap/`, `local.db`, `kv.db` *(migrating from a global root file)*, `chat_history.json` *(migrating from cache/)*. |
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
