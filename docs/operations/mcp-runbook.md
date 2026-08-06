# MCP Runbook

## 401

Check:

- token is current
- token belongs to the same `{owner}/{project}` endpoint
- Authorization header is exactly `Bearer {token}`
- proxy preserves `Authorization`

## Endpoint

Use:

```text
/api/projects/{owner}/{project}/mcp
```

Do not use a token from another project.

## After Reconnect

If reconnect works briefly then fails:

- inspect project MCP session state
- reset the token
- verify configured base URL
- confirm reverse proxy routes to the correct worker/project instance

## Useful Checks

- `version`
- `start_here`
- `help(topic="platform/api")`
- `help(topic="pipeline")`
