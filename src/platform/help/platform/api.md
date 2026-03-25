# Zebflow REST API Reference

All project endpoints require authentication (session cookie or Bearer token).
Base path: `/api/projects/{owner}/{project}`

## Authentication

Cookie-based: Set `zebflow_session={owner}` cookie (development mode).
MCP sessions: Use `Authorization: Bearer {token}` header.

## Projects

```
GET  /api/projects/{owner}              — list owner's projects
POST /api/projects/{owner}              — create project
GET  /api/projects/{owner}/{project}    — get project details
```

## Pipelines

```
GET    /api/projects/{o}/{p}/pipelines                      — list registry (tree view)
POST   /api/projects/{o}/{p}/pipelines                      — upsert pipeline
GET    /api/projects/{o}/{p}/pipelines/file?rel_path=...    — read pipeline source
PUT    /api/projects/{o}/{p}/pipelines/file                 — save pipeline source
DELETE /api/projects/{o}/{p}/pipelines/file                 — delete pipeline
POST   /api/projects/{o}/{p}/pipelines/activate             — activate pipeline for live traffic
POST   /api/projects/{o}/{p}/pipelines/deactivate           — deactivate pipeline
POST   /api/projects/{o}/{p}/pipelines/execute              — manually execute pipeline
```

Request body for upsert:
```json
{
  "name": "my-pipeline",
  "virtual_path": "/",
  "title": "My Pipeline",
  "trigger_kind": "webhook",
  "source": "{...pipeline JSON...}"
}
```

## Templates

```
GET  /api/projects/{o}/{p}/templates                    — list workspace tree
GET  /api/projects/{o}/{p}/templates/file?rel_path=...  — read template file
PUT  /api/projects/{o}/{p}/templates/file               — save template file
POST /api/projects/{o}/{p}/templates/create             — create new template
DELETE /api/projects/{o}/{p}/templates/file             — delete template
POST /api/projects/{o}/{p}/templates/compile            — compile check
```

## Sekejap Tables

```
GET    /api/projects/{o}/{p}/tables                 — list tables
POST   /api/projects/{o}/{p}/tables                 — create table
GET    /api/projects/{o}/{p}/tables/{table}         — get table details
DELETE /api/projects/{o}/{p}/tables/{table}         — delete table
POST   /api/projects/{o}/{p}/tables/rows            — upsert row
POST   /api/projects/{o}/{p}/tables/query           — query rows
```

Create table body:
```json
{
  "table": "blog_posts",
  "title": "Blog Posts",
  "hash_indexed_fields": ["slug", "status"],
  "range_indexed_fields": ["created_at"]
}
```

## Credentials

```
GET    /api/projects/{o}/{p}/credentials                 — list credentials
POST   /api/projects/{o}/{p}/credentials                 — create/update credential
GET    /api/projects/{o}/{p}/credentials/{id}            — get credential
DELETE /api/projects/{o}/{p}/credentials/{id}            — delete credential
```

## MCP Sessions

```
GET    /api/projects/{o}/{p}/mcp/session    — get current session
POST   /api/projects/{o}/{p}/mcp/session    — create session
DELETE /api/projects/{o}/{p}/mcp/session    — revoke session
```

Create session body:
```json
{
  "capabilities": ["pipelines.read", "pipelines.write", "templates.read", "tables.read"],
  "auto_reset_seconds": 86400
}
```

## Project Assistant

```
POST /api/projects/{o}/{p}/assistant/chat   — send message to assistant
GET  /api/projects/{o}/{p}/assistant/config — get assistant config
PUT  /api/projects/{o}/{p}/assistant/config — update assistant config
```

## Admin DB (superadmin only)

```
GET    /api/admin/db/collections            — list collections with counts
POST   /api/admin/db/query                  — run raw SekejapQL pipeline
GET    /api/admin/db/node/{slug}            — get node by slug
DELETE /api/admin/db/node/{slug}            — delete node by slug
```
