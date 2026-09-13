# Zebflow platform

Zebflow runs projects. A project is a git repository of pipelines and the
files they use, plus the data those pipelines create. Everything an agent or
a person does to a project goes through the same API the Studio uses.

## What a project contains

| | Where | What |
|---|---|---|
| **Pipelines** | `*.zf.json` in the source root | a trigger, nodes, edges — the executable graph (`help("pipeline")`) |
| **Pages and code** | `pages/*.tsx`, `components/*.tsx`, `scripts/*.ts`, `globals.css` | server-rendered TSX on the `zeb/react` runtime and the `zeb/ui` component set (`help("web")`) |
| **Docs** | `docs/*.md` | the project's own documents; `AGENTS.md`, `SOUL.md`, `MEMORY.md` are kept separately for agents |
| **Databases** | connections `default` (SQLite) and `default-multimodel` (Sekejap) in every project; PostgreSQL and others by credential | `help("db")` |
| **Credentials** | encrypted at rest under the instance key; referenced by id from nodes | Studio → Credentials |
| **Files** | ZebFS: `public/…` served anonymously at `/files/{owner}/{project}/…`, everything else private at `/fs/…` | `n.fs.*` nodes |
| **Configuration** | `zebflow.yaml` (layout, libraries, locks, policy), `zeb.lock` (installed node bundles and libraries) | Studio → Settings |

## Project layout

```
users/{owner}/{project}/
├── repo/                        the git repository — the project's source
│   ├── zebflow.yaml             ProjectConfiguration
│   ├── zeb.lock                 installed node bundles and zeb/* libraries
│   ├── globals.css              theme tokens, imported by pages
│   ├── api/…zf.json  pages/…tsx  components/  scripts/  jobs/     ← the source root is the repo root
│   ├── docs/                    project documents
│   ├── static/                  static assets, served at /static/{owner}/{project}/…
│   ├── schemas/sekejap/  schemas/sqlite/   declared schemas
│   ├── initial-data/            seed rows applied on install
│   ├── nodes/                   interfaces of installed third-party nodes
│   └── shared/ui/               components cloned from zeb/ui to own
├── data/
│   ├── store/                   sekejap/, local.db (SQLite), kv.db, assistant/{user}/memory.md
│   ├── cache/                   pipelines/ (live snapshots), agent_docs/ (AGENTS.md, SOUL.md), mapserver-artifacts/
│   ├── hub/                     nodes/, rwe-libraries/ — materialized installs
│   ├── logs/  recovery/
└── files/                       ZebFS objects
```

The source root is `repo/` itself. A project may point `spec.layout.source`
at a subdirectory, and pipelines, pages, assets and installs all follow; a
pipeline's identity (`file_rel_path`) is relative to the source root either
way. Other layout keys and their defaults: `static` = `static`, `docs` =
`docs`, `schema` = `schemas/sekejap`, `sqlite_schema` = `schemas/sqlite`,
`node_interfaces` = `nodes`. A new project starts with `zebflow.yaml`,
`zeb.lock`, `globals.css`, a README and one sample page + pipeline.

## Key concepts

- **Owner** — the user or organisation namespace: `/projects/{owner}/{project}`.
- **Activation** — a registered pipeline is a draft until activated; live traffic runs the activated snapshot, so edits after activation make it `stale` until activated again.
- **Webhook ingress** — `{method} /wh/{owner}/{project}{path}` for every active `trigger.webhook`.
- **MCP session** — a per-project token (`GET /api/projects/{o}/{p}/mcp/session`) that gives an agent the project's tools at `POST /api/projects/{o}/{p}/mcp`, narrowed by capabilities. `help("platform/agent")` is what that agent reads first.
- **Hub** — clonable packages (pipeline, template, folder and project bundles) and installable ones (node bundles, `zeb/*` libraries, recorded in `zeb.lock`). `help("guide/hub")`.
- **Offices and controller** — one instance is standalone; several form a cluster where a controller vouches for offices. `help("guide/federated-offices")`.

## API base

- Project resources: `/api/projects/{owner}/{project}/…` — `help("platform/api")`.
- Project list and creation: `/api/users/{owner}/projects`.
- Platform administration (superadmin): `/api/platform/…`.

Topics: `platform/api` (HTTP surface), `platform/operations` (layout, locks,
capabilities, probes), `platform/workflow` (an agent building a feature end
to end), `platform/agent` (the MCP instructions).
