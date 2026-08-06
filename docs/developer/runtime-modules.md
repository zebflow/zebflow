# Runtime Modules

This page maps top-level source modules to their internal responsibility.

| Module | Responsibility |
|---|---|
| `src/bin` | CLI/server entrypoints. |
| `src/platform` | Web app, APIs, services, auth, project management, Hub, MCP, settings, git, DB connections. |
| `src/pipeline` | Pipeline model, parser, validator, engine, expressions, node definitions, node dispatch. |
| `src/rwe` | Reactive web template compile, SSR, hydration, Tailwind processing, runtime protocol. |
| `src/language` | Sandboxed script runtime and tool initialization. |
| `src/mapserver` | Map layer publishing, source resolution, bbox/query/stats/tile/style handling. |
| `src/zebfs` | Project file storage, metadata, ACL, local backend. |
| `src/infra` | Execution backends, storage/cache abstractions, scheduler, health, cluster, websocket primitives. |
| `src/automaton` | Assistant/automation engines, tool calling, memory/planning interfaces. |
| `src/provision` | Deployment/provisioning helpers. |

For public user behavior, prefer the matching `docs/usage/` page. This page is for changing Zebflow internals.
