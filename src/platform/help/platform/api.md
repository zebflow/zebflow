# HTTP API

Everything the Studio does, an HTTP client can do. Base path for a project:
`/api/projects/{owner}/{project}`. Every response is JSON;
errors are `{"ok": false, "error": {"code": "…", "message": "…"}}`
(an unauthenticated request gets `401 {"error": "login required"}`).

## Authentication

`POST /login` with form fields `identifier` and `password` answers `303` and
sets `zebflow_session=<opaque token>`. Send that cookie on every call:

```bash
curl -s --cookie-jar /tmp/zf.txt -X POST http://localhost:10610/login -d "identifier=superadmin&password=…" -o /dev/null
curl -s -b /tmp/zf.txt http://localhost:10610/api/projects/acme/shop/pipelines
```

The cookie value is never the user name. MCP bearer tokens are for the MCP
endpoint only (`/api/projects/{o}/{p}/mcp`); REST routes do not accept them.

## Projects

```
GET    /api/users/{owner}/projects               list
POST   /api/users/{owner}/projects               create  { "project": "shop", "title": "Shop" }
DELETE /api/users/{owner}/projects/{project}     delete
```

## Pipelines

```
GET    /pipelines?path=&recursive=               list (source tree, optionally one folder)
GET    /pipelines/registry                        the registry index the Studio shows
GET    /pipelines/by-id?id=<file_rel_path>&include_source=true
POST   /pipelines/definition                      create or replace
DELETE /pipelines/definition                      { "file_rel_path": "api/posts.zf.json" }
POST   /pipelines/activate | /deactivate          { "file_rel_path": "api/posts.zf.json" }
POST   /pipelines/execute                         { "file_rel_path", "trigger"?: "webhook|schedule|manual", "input": {…}, "webhook_path"?, "webhook_method"?, "schedule_cron"? }
                                                  trigger absent: manual, unless the graph's first trigger is a webhook (then that route)
                                                  Accept: text/event-stream → event: signal …, event: result { ok, run_id, output | error }
POST   /pipelines/dsl                             { "dsl": "register api/posts | trigger.webhook … | …" }
POST   /pipelines/lock-toggle
GET    /pipelines/hits  ·  GET /pipelines/invocations
```

The definition body is the pipeline document as text:

```json
{ "file_rel_path": "api/posts.zf.json", "title": "Posts", "description": "", "trigger_kind": "webhook",
  "source": "{ \"apiVersion\": \"zebflow.com/v1\", \"kind\": \"Pipeline\", … }" }
```

Writing DSL is easier through `/pipelines/dsl` — put the JSON in a file and
send it with `-d @file`, because `--flags` do not survive shell quoting. The
same verbs the console accepts (`register`, `activate pipeline`,
`deactivate pipeline`, `execute pipeline`, `run`, `patch`, `get`, `describe`,
`git`) work there; `help("pipeline/dsl")`.

## Repository files

```
GET    /repo?path=&depth=&fields=                 the tree (everything, one folder, or paths only)
GET    /repo/file?path=pages/post.tsx             read  → { "file": { "content", "line_count", … } }
PUT    /repo/file?path=pages/post.tsx             write — the request body is the file's raw content
DELETE /repo/file?path=…
POST   /repo/folder  ·  POST /repo/move  ·  GET /repo/search
GET    /templates/outline?path=…                  imports/exports of a .tsx/.ts
POST   /templates/diagnostics                     { "rel_path", "content" } → compile diagnostics without saving
GET    /templates/git-status  ·  POST /templates/lock-toggle
POST   /rwe/cache/clear                           evict compiled pages
GET|POST /agent-docs  ·  /agent-docs/file          AGENTS.md, SOUL.md, MEMORY.md
```

Writing a component through `PUT /repo/file` evicts every compiled page that
inlined it.

## Databases

```
GET|POST /db/connections                          list · create
GET|PUT  /db/connections/{slug}  ·  POST /db/connections/test
GET    /db/connections/{id}/describe  ·  /schemas  ·  /tables  ·  /functions  ·  /table-preview
POST   /db/connections/{id}/tables                create a table
PUT|DELETE /db/connections/{id}/tables/{table}    alter · drop
POST   /db/connections/{id}/tables/{table}/rows   insert a row
POST   /db/connections/{id}/query                 run SQL / SekejapQL
GET|POST /tables  ·  PUT|DELETE /tables/{table}   the managed Sekejap tables (the Tables tab)
GET    /tables/schema/export  ·  POST /tables/schema/sync
GET|POST /db/sekejap/maintenance/{health,sync,compact}
```

A managed table:

```json
{ "table": "posts",
  "attributes": [ { "name": "slug", "kind": "string", "index_types": ["hash"] },
                  { "name": "body", "kind": "text", "index_types": ["fulltext"] } ],
  "hash_indexed_fields": ["status"], "range_indexed_fields": ["created_at"] }
```

Attribute kinds: `string` · `number` · `boolean` · `text` · `json` · `vector` · `geo`;
index types: `hash` · `range` · `fulltext` · `vector` · `spatial`.

## Credentials and files

```
GET|POST /credentials  ·  GET|PUT|DELETE /credentials/{id}  ·  GET /credentials/{id}/oauth/authorize  ·  GET /credential-types
GET    /files/list  ·  POST /files/upload  ·  /files/mkdir  ·  /files/rm  ·  PUT /files/access
GET    /files/object?ref=<path>                   one object's bytes, private or public, with the session or the MCP bearer; inline, never cached
GET    /files/{owner}/{project}/{*path}           (root path) public/… anonymously, the rest with a session
GET    /fs/{owner}/{project}/{*path}              (root path) private objects
```

Credential values are returned only to the owner's session (`GET /credentials/{id}`, for the Studio's edit form); the list carries `has_secret` only, and MCP, pipelines and pages never see a value — nodes reference a credential by id.

## Project services

```
GET    /git/status  ·  /git/health  ·  /git/branches   POST /git/commit  ·  /git/repair  ·  /git/branches   GET|PUT /git/remote
GET|POST /members  ·  /invites                    membership
GET|PUT  /settings/{section}                       general · git · members · policy · automatons · logs
GET    /nodes  ·  /nodes/by-kind/{kind}   POST /nodes/install  ·  /nodes/install/review  ·  /nodes/uninstall/{kind}
GET    /rwe/libraries   POST /rwe/libraries/enable  ·  /rwe/libraries/remove
GET    /install/catalog/ui   POST /install/ui   ·  /install/ui/review     the clone-to-own component catalog
GET    /dependencies  ·  /runtime  ·  /help  ·  /editor/completion-catalog   POST /reindex
GET|POST /hub/assets …                              help("guide/hub")
GET    /mapserver/{instance}/sources  ·  /layers  ·  /layers/{id}  ·  /layers/{id}/stats
POST   /assistant/chat   GET|PUT /assistant/config
```

## MCP session

```
GET    /mcp/session                → { "token", "mcp_url", "capabilities", "enabled", "created_at", "auto_reset_seconds" }
POST   /mcp/session                { "capabilities": ["project.read", "pipelines.read", "pipelines.write", …], "auto_reset_seconds": 86400 }
PUT    /mcp/session                { "enabled": false }
POST   /mcp/session/reset-token
DELETE /mcp/session
```

The agent then speaks MCP (streamable HTTP) at `mcp_url` with
`Authorization: Bearer <token>` and `Accept: application/json, text/event-stream`.
Capabilities: `help("platform/operations")`.

## Platform administration (superadmin)

```
GET|POST /api/platform/users
GET    /api/platform/db/collections   POST /api/platform/db/query   GET|DELETE /api/platform/db/node/{slug}
GET    /api/platform/credentials/keyring   POST /api/platform/credentials/{rotate,rekey,reencrypt}
       /api/platform/hub/…                 service, publishers, repositories, tokens, grants, install
       /api/platform/cluster/…             join tokens, vouch, break-glass;  /api/platform/office/…
```

Webhook ingress is not under `/api`: `{method} /wh/{owner}/{project}{path}`.
Public map tiles and layers are served at `/ms/{owner}/{project}/…`, static
assets at `/static/{owner}/{project}/…`.
