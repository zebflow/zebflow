# MCP by Project

MCP access in Zebflow is project-scoped: one server per project, at
`POST /api/projects/{owner}/{project}/mcp`, streamable HTTP with
`Authorization: Bearer <token>` and `Accept: application/json, text/event-stream`
(the request is refused with 406 without that header).

The token comes from `GET /api/projects/{owner}/{project}/mcp/session`; the
same route accepts `PUT` to toggle MCP on/off for the project and `DELETE` to
remove the session, and `POST .../mcp/session/reset-token` mints a new token.

Every active pipeline built with an `n.trigger.mcp` node is exposed as an
additional callable tool on that project's MCP server, alongside the fixed
tool set (`pipeline_register`, `pipeline_execute`, `file_read`, `help`, and
the rest — see `help("platform/agent")`).

That matters because agents should operate with clear boundaries:

- one owner
- one project
- one capability envelope

## Why this is important

It makes agent access:

- easier to reason about
- easier to revoke
- easier to audit
- cheaper to operate safely

For the deep operational reference, see `help("platform/agent")`.
