# HTTP API Reference

Zebflow has public project routes and authenticated management routes.

## Public Runtime Routes

- `/wh/{owner}/{project}/...` serves active webhook pipelines.
- `/fs/{owner}/{project}/...` serves ZebFS objects.
- `/ms/{owner}/{project}/...` serves map services.
- `/ws/{owner}/{project}/...` serves WebSocket routes.

## Management Routes

Project management APIs start with `/api/projects/{owner}/{project}`. They cover
pipelines, templates, files, credentials, connections, databases, Git, settings,
Hub, runtime operations, and MCP sessions.

Platform Hub APIs start with `/api/hub`. Platform management routes require the
matching platform permission.

## Response Shape

Successful JSON responses use a clear result object. Errors use an object with a
stable `code` and a human readable `message`. Secret values must not appear in
list responses, logs, or normal errors.

## Authentication

Browser management uses a session cookie. API and MCP clients use a scoped
token where supported. Project routes must check both identity and project
capability before doing work.

The exact route list should be generated from the router. Until that generator
exists, `src/platform/web/mod.rs` is the route source.
