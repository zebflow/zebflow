## Milestone 1 Platform ✓ ACHIEVED

### Unified Management Interface ✓
- **Adapter pattern implemented**: REST API, MCP tools, and future assistant interfaces all call the same `ProjectService` methods
- **Unified authorization**: All entry points use `AuthorizationService.ensure_project_capability()` with `ProjectAccessSubject` (user, mcp_session, assistant_profile)
- **Capability-based access**: MCP tool names map to `ProjectCapability` enums via `mcp_tool_capability()` function
- **Thin transport adapters**: Each transport only handles auth, validation, service calls, and output formatting

### Project-Scoped MCP Server ✓
- **Session management**: In-memory `McpSessionService` with one active token per project
- **User-controlled toggle**: UI in project header allows users to create/revoke sessions
- **Temporary tokens**: 64-char hex tokens valid until revoked or server restart
- **Capability selection**: Users choose which operations to allow (e.g., `pipelines.read`, `templates.write`)
- **Policy isolation**: Each session creates ephemeral project policy and binding (auto-cleaned on revocation)
- **MCP endpoint**: Per-project at `/api/projects/{owner}/{project}/mcp` with Bearer token auth
- **Tool implementations**: 
  - `list_pipelines` (PipelinesRead)
  - `list_templates` (TemplatesRead)
  - More tools ready to add: get/upsert pipelines, templates, credentials, tables, etc.

### Integration Complete ✓
- **Capability checks**: `McpSessionCreate`, `McpSessionRevoke` enforced on session API endpoints
- **Token validation**: MCP handler validates tokens and injects session context
- **Subject-based authz**: MCP sessions are first-class subjects in authorization service
- **Documentation**: Adapter pattern fully documented in `docs/PLATFORM_WEB.md`
- **Compile-time safety**: All implementations pass `cargo check`

### MCP Transport Integration ✓
- **rmcp StreamableHttpService**: Proper MCP protocol implementation with JSON response mode
- **Middleware injection**: Token validation and session injection via Axum middleware
- **Extension-based access**: Tools access session via `Extension<http::request::Parts>`
- **Working tools**: `list_pipelines` and `list_templates` functional
- **Capability enforcement**: Each tool checks required capability before execution

### Next Steps (Milestone 2 prep)
- [ ] Add more MCP tools (get/create/update/delete operations, pipeline execution)
- [ ] Token hashing + TTL/expiration
- [ ] Rate limiting per session
- [ ] Audit log for MCP tool calls
- [ ] Enable SSE mode for long-running operations

=====================================
## Milestone 2

- platform-level smart routing
    - change admin n login main path
    - reroute path to another path
    - lightspeed static / cacheable configuration
- security-first under platform
    - Server security barrier 
    - Zero access policy
        -> mcp, rest api, internal llm
- discovery-first under platform
    - SEO related based on webhook that is seoable
- pipeline runtime registry
    - current working tree = draft
    - activated snapshot = production
    - hot reload by atomic compiled registry replacement
    - scheduler derived from active registry, not from mutable files
