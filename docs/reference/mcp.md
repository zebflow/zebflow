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

- `template_list`
- `template_get`
- `template_create`
- `template_write`
- `template_search`
- `template_edit`
- `template_outline`
- `template_deps`
- `template_batch_edit`

## Project Knowledge and Services

- `docs_project_list`
- `docs_project_read`
- `docs_project_write`
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

Related source:

- `src/platform/mcp/handler.rs`
- `src/platform/services/mcp_session.rs`
- `src/platform/services/assistant_tools.rs`
