# Zebflow Coverage Map

This file maps source areas to documentation and skill references. It is not the full manual; it is the checklist that prevents undocumented surfaces.

## Rule

- User-operated surfaces belong in `docs/usage/`.
- Runtime and extension internals belong in `docs/developer/`.
- Running, deployment, health, backups, and incident response belong in `docs/operations/`.
- Working procedure and routing guidance belongs in `skills/zebflow/references/`.
- Embedded live help under `src/platform/help/` remains the detailed in-product authority for pipeline, web, platform, DB, Hub, and node reference material.

## Source Coverage

| Source area | What it owns | User docs | Developer docs | Skill reference |
|---|---|---|---|---|
| `src/bin` | CLI/server entrypoints | `docs/usage/install.md`, `docs/usage/deployment.md` | `docs/developer/runtime-modules.md` | `distribution.md`, `runtime-workflows.md` |
| `src/platform` | web app, APIs, services, auth, settings, git, Hub, MCP, project operations | `docs/usage/project-studio.md`, `docs/usage/hub.md`, `docs/usage/mcp.md` | `docs/developer/platform-internals.md`, `docs/developer/mcp-contract.md`, `docs/developer/policy.md` | `platform-internals.md`, `mcp-workflows.md`, `hub-packages.md` |
| `src/pipeline` | pipeline model, DSL, engine, nodes, expressions | `docs/usage/pipelines.md`, `docs/usage/pipeline-language.md`, `docs/usage/pipeline-examples.md` | `docs/developer/pipeline-runtime.md`, `docs/developer/node-definitions.md` | `pipeline-authoring.md`, `node-authoring.md` |
| `src/rwe` | reactive web templates, SSR, hydration, Tailwind compiler | `docs/usage/templates.md` | `docs/developer/runtime-modules.md` | `web-authoring.md` |
| `src/language` | sandboxed script engine | `docs/usage/pipeline-language.md` | `docs/developer/runtime-modules.md` | `pipeline-authoring.md`, `platform-internals.md` |
| `src/mapserver` | map layer registry, tiles, bbox/query/stats, styles | `docs/usage/mapserver-gis.md` | `docs/developer/runtime-modules.md` | `data-files-maps.md` |
| `src/zebfs` | project file storage and ACL | `docs/usage/files-storage.md` | `docs/developer/runtime-modules.md` | `data-files-maps.md` |
| `src/infra` | storage abstractions, execution backends, cluster, scheduler, health, transport | `docs/usage/deployment.md` | `docs/developer/runtime-modules.md` | `platform-internals.md`, `runtime-workflows.md` |
| `src/automaton` | assistant/automation engines and tool calling | `docs/usage/mcp-ai-tools.md` | `docs/developer/runtime-modules.md` | `platform-internals.md` |
| `src/provision` | Kubernetes/provisioning helpers | `docs/operations/README.md`, `docs/operations/health-runtime.md` | `docs/developer/runtime-modules.md` | `distribution.md` |
| `libraries` | Zeb frontend libraries | `docs/usage/templates.md` | `docs/developer/runtime-modules.md` | `web-authoring.md`, `distribution.md` |
| `composites` | built-in composite node packages | `docs/usage/pipelines.md` | `docs/developer/composite-nodes.md` | `node-authoring.md`, `distribution.md` |
| `npm` | npm installer/package | `docs/usage/install.md` | `docs/developer/runtime-modules.md` | `distribution.md` |
| `pip` | pip installer/package | `docs/usage/install.md` | `docs/developer/runtime-modules.md` | `distribution.md` |
| `docker`, `charts`, `k8s` | container and cluster deployment | `docs/usage/deployment.md`, `docs/operations/README.md` | `docs/developer/runtime-modules.md` | `distribution.md` |
| `integrations` | integration-specific assets or adapters when present | `docs/usage/deployment.md` | `docs/developer/runtime-modules.md` | `distribution.md` |
| `.github` | CI and release automation | `docs/operations/README.md` | `docs/developer/runtime-modules.md` | `quality-checks.md`, `distribution.md` |
| `tests` | integration, platform, language, RWE, and benchmark fixtures | none | `docs/developer/runtime-modules.md` | `quality-checks.md` |
| `docs` | committed user, developer, and operations documentation | `docs/README.md` | `docs/coverage-map.md` | `editing-rules.md` |
| `skills` | repository working instructions and task routing | none | `docs/coverage-map.md` | `editing-rules.md` |
| `data` | local runtime/project data when present | `docs/usage/files-storage.md`, `docs/usage/sekejap-db.md` | `docs/developer/platform-internals.md` | `project-surfaces.md`, `data-files-maps.md` |
| `runtime` | runtime support assets when present | `docs/usage/deployment.md` | `docs/developer/runtime-modules.md` | `runtime-workflows.md` |
| `.ignored` | local scratch, archived work, generated previews, and non-committed notes | none | `docs/coverage-map.md` | `project-surfaces.md`, `editing-rules.md` |

## In-Product Help Coverage

| Embedded help | Covers |
|---|---|
| `src/platform/help/pipeline/` | Pipeline mental model, DSL, authoring, web responses, examples, live node catalog |
| `src/platform/help/web/` | TSX pages, hooks, Tailwind, design system, libraries, markdown, PDF, deck.gl |
| `src/platform/help/platform/` | REST API, operations, workflow, assistant/automation |
| `src/platform/help/db/` | DB and Sekejap guidance |
| `src/platform/help/guide/hub/` | Hub concepts, packages, nodes, frontend libraries |
| `src/platform/help/tool/` | Template/script utility library |

## Node Family Coverage

The authoritative node list is generated from Rust `NodeDefinition` values. Use `help(topic="pipeline/nodes")` for the full live list and `help(topic="pipeline/nodes/{kind}")` for one node.

Current node families from `src/pipeline/nodes/basic`:

- triggers: webhook, function, schedule, manual, websocket, websocket client, KV subscribe, MCP, web error
- logic: if, match, collect, foreach, reduce, retry
- data: PostgreSQL, SQLite, Sekejap query, Sekejap insert
- files: fs list/head/get/put/delete/copy/move/mkdir/save/thumbnail/compress/decompress/pdf convert
- tables and geo: table convert/query, geo inspect/convert
- mapserver: publish, unpublish, get, list
- web: response, static generate, static site, docs generate
- realtime: ws emit, sync state, client send
- AI/media: ai agent, TTS
- utility: script, HTTP request, browser run, crypto, function call, KV operations

## Missing Coverage Rule

When a feature does not fit the existing docs, add the smallest new page under the correct audience folder and link it from this map. Do not leave behavior documented only in conversation history.
