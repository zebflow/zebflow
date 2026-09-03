# MCP Reference

MCP gives a tool client controlled access to one Zebflow project. The token and
project route set the scope.

## Discovery

- `start_here`
- `version`
- `help`
- `help_search`

## Pipelines

- `pipeline_list`
- `pipeline_search`
- `pipeline_get`
- `pipeline_register`
- `pipeline_describe`
- `pipeline_patch`
- `pipeline_activate`
- `pipeline_deactivate`
- `pipeline_execute`
- `pipeline_run`
- `pipeline_get_invocations`

## Templates

- `file_list`
- `file_read`
- `file_create`
- `file_write`
- `file_search`
- `file_edit`
- `file_outline`
- `file_deps`
- `file_batch_edit`

## Project Knowledge and Services

- `file_list`
- `file_read`
- `file_write`
- `docs_agent_list`
- `docs_agent_read`
- `docs_agent_write`
- `connection_list`
- `connection_describe`
- `credential_list`
- `git_command`
- `list_ui_catalog`
- `install_ui_components`
- `move_resource`

Every tool call checks its project capability. Tool descriptions and argument
schemas belong in the MCP tool definition, not only in a guide.

## Create a Custom TypeScript Module

Read the web rules before writing source:

```text
help topic=web/custom-scripts
```

Then scaffold, inspect, and replace the script:

```text
file_create kind=script name=format-address
file_read rel_path=scripts/format-address.ts
file_write rel_path=scripts/format-address.ts content="<complete TypeScript source>"
```

Use `file_create` first so the destination follows the project layout. Keep
exports camelCase, use `@/` for local imports, and do not add npm, JSR, CDN,
React, Preact, or Node package imports. For complex existing behavior, search
Hub packages or use a project-enabled `zeb/*` library.

After writing, use `file_read` to verify the saved source and compile or run
the page that imports it. Never place credentials or private tokens in a web
script.

Related source:

- `src/platform/mcp/handler.rs`
- `src/platform/services/mcp_session.rs`
- `src/platform/services/assistant_tools.rs`
