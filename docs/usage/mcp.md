# MCP

MCP access is project-scoped.

Endpoint:

```text
/api/projects/{owner}/{project}/mcp
```

Session API:

```text
GET    /api/projects/{owner}/{project}/mcp/session
POST   /api/projects/{owner}/{project}/mcp/session
DELETE /api/projects/{owner}/{project}/mcp/session
```

Detailed in-product help:

- `help(topic="platform/agent")`
- `help(topic="platform/api")`
- `help(topic="pipeline")`
- `help(topic="web")`

## Typical Flow

1. Enable a project MCP session.
2. Copy the endpoint and token into the client.
3. Call `start_here`.
4. Use `help` and `help_search` before writing pipelines or templates.
5. Use project-scoped tools to inspect, edit, run, and verify.

## Project Boundary

Tokens belong to one project. A token for one project must not be treated as a token for another project.

## Pipeline Work

For pipeline creation through MCP:

1. Use `help(topic="pipeline/dsl")`.
2. Use `help(topic="pipeline/nodes/{kind}")` before using unfamiliar node flags.
3. Register the pipeline.
4. Activate it only when it should receive live traffic.
5. Execute or call its webhook URL to verify behavior.
