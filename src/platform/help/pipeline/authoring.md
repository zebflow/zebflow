# Pipeline Authoring

Pipelines are directed graphs stored as `.zf.json` files under `repo/pipelines/`.
Use the **DSL** (`pipeline_register`) to author them — the JSON is auto-generated.
Read this doc for the underlying model. See `help("pipeline/dsl")` for the DSL.

> **Before you write a pipeline node that references a template or credential:**
> - `web.response --template <path>` — the path must be an exact `rel_path` from `template_list` (e.g. `pages/home.tsx` — always ends in `.tsx`). Call it if you don't have the value in your current context.
> - `--credential <slug>` — the slug must be exact from `connection_list`. Call it if unsure.
> Never guess these values. A wrong template path silently serves nothing; a wrong credential slug causes auth failures.

---

## File Location

```
pipelines/
  api/
    auth/login.zf.json
    posts/list.zf.json
  pages/
    home.zf.json
    auth/login.zf.json
  jobs/
    daily-report.zf.json
```

Naming convention: `pipelines/<virtual-path>/<name>.zf.json`

---

## JSON Model

Every pipeline compiles to a `PipelineGraph` object:

```json
{
  "kind": "zebflow.pipeline",
  "version": "0.1",
  "id": "auth-login",
  "entry_nodes": ["a"],
  "nodes": [
    {
      "id": "a",
      "kind": "n.trigger.webhook",
      "input_pins": [],
      "output_pins": ["out"],
      "config": { "path": "/api/auth/login", "method": "POST" }
    },
    {
      "id": "b",
      "kind": "n.pg.query",
      "input_pins": ["in"],
      "output_pins": ["out"],
      "config": {
        "credential_id": "main-db",
        "query": "SELECT * FROM users WHERE identifier = $1",
        "params_path": "identifier"
      }
    },
    {
      "id": "c",
      "kind": "n.web.response",
      "input_pins": ["in"],
      "output_pins": ["out", "error"],
      "config": { "template": "pages/auth/login.tsx" }
    }
  ],
  "edges": [
    { "from_node": "a", "from_pin": "out", "to_node": "b", "to_pin": "in" },
    { "from_node": "b", "from_pin": "out", "to_node": "c", "to_pin": "in" }
  ]
}
```

### Key fields

| Field | Type | Description |
|---|---|---|
| `kind` | string | Always `"zebflow.pipeline"` |
| `version` | string | Always `"0.1"` |
| `id` | string | Pipeline slug (matches filename without `.zf.json`) |
| `entry_nodes` | string[] | IDs of nodes with no incoming edges (auto-computed by DSL) |
| `nodes` | Node[] | All nodes in the graph |
| `edges` | Edge[] | All pin-to-pin connections |

### Node fields

| Field | Type | Description |
|---|---|---|
| `id` | string | Node label (e.g. `"a"`, `"b"`, `"trigger"`, `"n0"`) |
| `kind` | string | Full node kind (e.g. `"n.trigger.webhook"`, `"n.pg.query"`) |
| `input_pins` | string[] | Usually `["in"]` or `[]` for triggers |
| `output_pins` | string[] | Usually `["out"]`; logic nodes have named pins |
| `config` | object | Node-specific configuration (credential IDs, SQL, template paths, etc.) |

### Edge fields

| Field | Type | Description |
|---|---|---|
| `from_node` | string | Source node ID |
| `from_pin` | string | Output pin name (`"out"` or named pin like `"true"`, `"false"`) |
| `to_node` | string | Target node ID |
| `to_pin` | string | Input pin name (usually `"in"`) |

---

## Pipeline Lifecycle

| Status | Meaning |
|---|---|
| `draft` | Registered but never activated — not serving traffic |
| `active` | Live; active snapshot matches current source |
| `stale` | Live but source changed since last activation — needs re-activate |
| `inactive` | Explicitly deactivated; source retained |

Workflow: `register` → `activate` → (edit → `activate` again to unstale)

---

## Webhook Ingress URL

When activated, webhook pipelines receive traffic at:

```
{METHOD} /wh/{owner}/{project}/{configured-path}
```

Example: `trigger.webhook --path /api/auth/login --method POST` →

```
POST /wh/superadmin/my-project/api/auth/login
```

---

## Node Kind Reference

All node kinds use the `n.` prefix. Short aliases work in DSL (e.g. `pg.query` → `n.pg.query`).

| Kind | Short alias | Input pins | Output pins |
|---|---|---|---|
| `n.trigger.webhook` | `trigger.webhook` | _(none)_ | `out` |
| `n.trigger.schedule` | `trigger.schedule` | _(none)_ | `out` |
| `n.trigger.manual` | `trigger.manual` | _(none)_ | `out` |
| `n.script` | `script` | `in` | `out` |
| `n.pg.query` | `pg.query` | `in` | `out` |
| `n.http.request` | `http.request` | `in` | `out` |
| `n.web.response` | `web.response` | `in` | `out`, `error` |
| `n.sekejap.query` | `sekejap.query` | `in` | `out` |
| `n.auth.token.create` | `auth.token.create` | `in` | `out` |
| `n.logic.if` | `logic.if` | `in` | `true`, `false` |
| `n.logic.switch` | `logic.switch` | `in` | _(named cases)_ |
| `n.logic.branch` | `logic.branch` | `in` | _(named branches)_ |
| `n.logic.merge` | `logic.merge` | _(named)_ | `out` |
| `n.ws.emit` | `ws.emit` | `in` | `out` |
| `n.ws.sync_state` | `ws.sync_state` | `in` | `out` |
| `n.crypto` | `crypto` | `in` | `out` |
| `n.ai.agent` | `ai.agent` | `in` | `out` |

---

## Config Key Reference

Key config fields and their DSL flag equivalents:

| Node | Config key | DSL flag | Description |
|---|---|---|---|
| `n.pg.query` | `credential_id` | `--credential` | PostgreSQL credential ID |
| `n.pg.query` | `query` | `-- <sql>` (body) | SQL query |
| `n.pg.query` | `params_path` | `--params-path` | Dot-notation path into upstream payload for `$1`/`$2` binds. e.g. `"identifier"` |
| `n.pg.query` | `params_expr` | `--params-expr` | JS expression returning array of bind params. e.g. `"[input.id, input.name]"` |
| `n.script` | `source` | `-- <code>` (body) | Script source code |
| `n.web.response` | `template` | `--template` | TSX path relative to `templates/`, e.g. `pages/home.tsx` (`.tsx` extension optional) |
| `n.web.response` | `location` | `--location` | Redirect URL; supports `$.field` for dynamic resolution from payload |
| `n.web.response` | `set_cookie` | `--set-cookie` | Cookie spec string: `name=X,value=$.token,http-only,max-age=86400` |
| `n.web.response` | `status` | `--status` | HTTP status code |
| `n.trigger.webhook` | `auth_type` | `--auth-type` | `none`, `jwt`, `hmac`, `api_key` |
| `n.trigger.webhook` | `auth_credential` | `--auth-credential` | Credential ID for auth verification |
| `n.auth.token.create` | `credential_id` | `--credential` | JWT signing key credential ID |
| `n.auth.token.create` | `expires_in` | `--expires-in` | Token lifetime in seconds |

---

## Webhook Input Shape

Body fields are always merged to root — regardless of encoding. Path params and query string are nested. This means a pipeline works the same whether the client sends JSON, a form POST, or a multipart upload.

### `application/json`

```json
{ "email": "user@example.com", "password": "secret" }
```

→ `input.email`, `input.password`

### `application/x-www-form-urlencoded` (native HTML form POST)

```
email=user%40example.com&password=secret
```

→ same: `input.email`, `input.password` — percent-decoded automatically

### `multipart/form-data` (file upload)

Text fields merge to root. Files go under `input.files.{field_name}`:

```json
{
  "email": "user@example.com",
  "files": {
    "avatar": {
      "filename": "photo.jpg",
      "content_type": "image/jpeg",
      "size": 12345,
      "data": "<base64>"
    }
  }
}
```

`data` is base64-encoded — pipe it to a script node to store, forward, or process.

### Always present

```json
{
  "params": { "id": "42" },
  "query": { "page": "1" },
  "auth": { "player_id": "...", "roles": [] }
}
```

`auth` is injected by the webhook trigger when `--auth-type jwt` is configured and the token is valid.
