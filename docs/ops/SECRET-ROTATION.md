# Secret Rotation Runbook

Use this after security fixes land and before exposing a Docker image publicly.

## Platform Env

Generate fresh deployment secrets outside the image:

```sh
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
export ZEBFLOW_CLUSTER_JOIN_TOKEN="$(openssl rand -hex 32)"
export ZEBFLOW_SECRET_ROTATION_EPOCH="$(date +%s)"
```

`ZEBFLOW_SECRET_ROTATION_EPOCH` invalidates persisted platform-issued tokens created before that Unix timestamp. Web sessions are in-memory, so restarting the container clears them.

Do not use `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=secret`. The binary rejects it unless `ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD=1` is explicitly set for disposable local development.

## Platform-Issued Tokens

After restart:

- Recreate MCP sessions from the project UI/API.
- Recreate hub publisher/read tokens that are used by deployed projects.
- Reconnect offices with the new `ZEBFLOW_CLUSTER_JOIN_TOKEN` if controller/office mode is enabled.

## External Provider Secrets

Rotate these in their source systems, then update the matching project credentials:

- Browserless tokens.
- API provider tokens.
- JWT signing keys.
- Webhook HMAC/API keys.
- Database credentials, if any were shared during the vulnerable window.

## Verification

- Old web cookies should require login after restart.
- Old MCP bearer tokens should return `401`.
- MCP tokens created before `ZEBFLOW_SECRET_ROTATION_EPOCH` should no longer work.
- New MCP tokens should work only on their own `{owner}/{project}` MCP endpoint.
