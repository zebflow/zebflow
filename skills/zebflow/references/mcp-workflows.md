# MCP Workflows

Authoritative code:

- `src/platform/mcp/handler.rs`
- `src/platform/services/mcp_session.rs`
- `src/platform/help/guide/**/mcp-by-project.md`
- `src/platform/help/platform/*.md`
- `src/platform/help/platform/api.md`

Project MCP endpoint:

```text
/api/projects/{owner}/{project}/mcp
```

Session management:

```text
GET    /api/projects/{owner}/{project}/mcp/session
POST   /api/projects/{owner}/{project}/mcp/session
DELETE /api/projects/{owner}/{project}/mcp/session
```

Working order:

1. Call `start_here`.
2. Call `help` with the needed topic.
3. List or inspect the artifact before editing it.
4. Write or patch the smallest necessary surface.
5. Activate only when the artifact is meant to serve live traffic.
6. Run or call the route that proves the change.

Core tools by surface:

| Surface | Tools |
|---|---|
| Orientation | `start_here`, `version`, `help`, `help_search` |
| Pipelines | `pipeline_list`, `pipeline_search`, `pipeline_get`, `pipeline_register`, `pipeline_describe`, `pipeline_patch`, `pipeline_activate`, `pipeline_deactivate`, `pipeline_execute`, `pipeline_run`, `pipeline_get_invocations` |
| Templates | `template_list`, `template_get`, `template_search`, `template_outline`, `template_deps`, `template_create`, `template_write`, `template_edit`, `template_batch_edit` |
| Docs | `docs_project_read`, `docs_project_write`, instruction-document read/write tools |
| Connections | `connection_list`, `connection_describe`, query tools |
| Git | `git` subcommands exposed through the project session |
| Add material | install/catalog and move tools where available |

Pipeline rule:

- Use `pipeline_register` for full pipeline creation or replacement.
- Use `pipeline_patch` only after `pipeline_describe`, and only for one node at a time.
- Long script/SQL bodies must go in the dedicated `body` field when the tool supports it.
- After `pipeline_register` or `pipeline_patch`, call `pipeline_activate` if the route/function/schedule should be live.

Help topics to call before authoring:

```text
pipeline
pipeline/dsl
pipeline/authoring
pipeline/web
pipeline/nodes
pipeline/nodes/{kind}
web
web/tailwind
web/libraries
db
platform/api
platform/operations
```
