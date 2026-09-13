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
POST   /api/projects/{owner}/{project}/mcp/session              { capabilities, auto_reset_seconds }
PUT    /api/projects/{owner}/{project}/mcp/session              { enabled }
POST   /api/projects/{owner}/{project}/mcp/session/reset-token
DELETE /api/projects/{owner}/{project}/mcp/session
```

The tool list is what the server answers to `tools/list`; there are no
`template_*`, `docs_project_*` or raw-DSL tools. `platform/agent.md` is the
instructions text every MCP client receives.

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
| Files (pages, components, scripts, CSS, docs) | `file_list`, `file_read`, `file_search`, `file_outline`, `file_deps`, `file_create`, `file_write`, `file_edit`, `file_batch_edit`, `move_resource` |
| Agent docs (AGENTS.md, SOUL.md, MEMORY.md) | `docs_agent_list`, `docs_agent_read`, `docs_agent_write` |
| Data | `connection_list`, `connection_describe`, `credential_list` — queries run through `pipeline_run` |
| Git | `git_command` with `subcommand` status · log · diff · add · commit |
| UI kit | `list_ui_catalog`, `install_ui_components` |

Pipeline rule:

- Use `pipeline_register` for full pipeline creation or replacement.
- Use `pipeline_patch` only after `pipeline_describe`, and only for one node at a time.
- Long script/SQL bodies go in the DSL body (`-- "…"`); `pipeline_register` takes the whole DSL as `body`.
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
