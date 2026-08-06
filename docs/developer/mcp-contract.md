# MCP Contract

Authority:

- `src/platform/mcp/handler.rs`
- `src/platform/services/mcp_session.rs`
- `src/platform/model.rs`
- `src/platform/help/platform/api.md`

Endpoint:

```text
/api/projects/{owner}/{project}/mcp
```

Auth:

- Bearer token
- project-scoped session
- capability-checked tools

Core tool groups:

- orientation and help
- pipelines
- templates
- docs
- database connections
- git
- install/add surfaces
- dynamic `n.trigger.mcp` function tools

Contract rule:

MCP tools should delegate to platform services. They should not invent a separate behavior path from UI/API behavior.
