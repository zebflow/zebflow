# MCP by Project

MCP access in Zebflow is project-scoped: one server per project, at
`POST /api/projects/{owner}/{project}/mcp`, streamable HTTP with
`Authorization: Bearer <token>` and `Accept: application/json, text/event-stream`
(the request is refused with 406 without that header).

The token comes from `GET /api/projects/{owner}/{project}/mcp/session`; the
same route accepts `PUT` to toggle MCP on/off for the project and `DELETE` to
remove the session, and `POST .../mcp/session/reset-token` mints a new token.

That server is the project's **dev** MCP: the fixed tool set an agent uses to
build the project (`pipeline_register`, `pipeline_execute`, `file_read`,
`help`, and the rest — see `help("platform/agent")`). It answers on the
platform address only, never on a project host.

An **app** publishes its own functions separately: every active
`trigger.mcp` with the same `--route` is one tool of a published MCP server on
the `mcp` surface — `/_mcp/ROUTE` on the project's hosts and
`/mcp/{owner}/{project}/ROUTE` on the platform, off by default (Settings →
Addressing), behind the `--auth` each route declares. A published tool is
never listed or callable on the dev MCP, and a published server lists nothing
but its own tools. In 0.11 the declaration is checked; the routes are served
from 0.11.1.

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
