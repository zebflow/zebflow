# Pipeline authoring

A pipeline is a `.zf.json` document in the project's source root — the
repository root, unless `zebflow.yaml` sets `spec.layout.source`. Author it
with the DSL (`pipeline_register` over MCP, or `register …` in the project
console); the JSON is generated. This page is the underlying model.
The DSL itself: `help("pipeline/dsl")`.

> Before a node references something by name, read the real value:
> - `web.response.send --template <path>` — an exact `rel_path` from `file_list`, ending in `.tsx`. A wrong path is a 500 at request time.
> - `--credential <id>` — an exact id from `credential_list`, on every node that takes one (a database, a JWT signing key, an smtp relay, a provider). Connection slugs from `connection_list` are for `connection_describe`, never for `--credential`.

`pipeline_list` and `file_list` are indexes; `pipeline_search` / `file_search`
grep contents; `pipeline_get` / `file_read` / `file_outline` open one file.

---

## Where files go

```
api/posts.zf.json          pipeline (file_rel_path = "api/posts.zf.json")
jobs/daily-report.zf.json
pages/post.tsx             template the pipeline renders
components/post-card.tsx
scripts/slugify.ts
globals.css
docs/schema.md
```

The folder is the author's choice; `api/`, `pages/`, `jobs/` is the
convention. The identifier is the path relative to the source root, so
moving the source root never renames a pipeline. `.zf.json` may be omitted
when you name one.

Template metadata is optional and helps `file_list query=…` find a page:

```tsx
/*
zebflow:
  title: Blog Home
  description: Public blog listing with featured posts and pagination.
  keywords: [blog, posts, pagination]
*/
```

---

## JSON model

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "Pipeline",
  "metadata": { "name": "api/login" },
  "spec": {
    "id": "api/login",
    "entry_nodes": ["n0"],
    "nodes": [
      { "id": "n0", "kind": "trigger.webhook", "input_pins": [], "output_pins": ["out"],
        "config": { "route": "/api/login", "method": "POST" } },
      { "id": "n1", "kind": "sekejap.query.run", "input_pins": ["in"], "output_pins": ["out"],
        "config": { "query": "SELECT * FROM users WHERE email = $1", "param": { "1": "{{ input.webhook.body.email }}" } } },
      { "id": "n2", "kind": "web.response.send", "input_pins": ["in"], "output_pins": ["out"],
        "config": { "template": "pages/login.tsx" } }
    ],
    "edges": [
      { "from_node": "n0", "from_pin": "out", "to_node": "n1", "to_pin": "in" },
      { "from_node": "n1", "from_pin": "out", "to_node": "n2", "to_pin": "in" }
    ]
  }
}
```

| Field | Meaning |
|---|---|
| `metadata.name`, `spec.id` | the pipeline's identity — the same value, equal to `file_rel_path` without the extension |
| `spec.entry_nodes` | nodes with no incoming edge; computed by the DSL |
| `spec.nodes[].id` | `n0, n1, …` in pipe mode; the `[label]` you wrote in graph mode |
| `spec.nodes[].kind` | the kind exactly as the DSL writes it: `family.noun.verb`, `trigger.<source>`, `input.<type>`, `logic.<verb>`, or `x.<package>.<noun>.<verb>` |
| `spec.nodes[].input_pins` / `output_pins` | `[]`/`["out"]` for triggers, `["in"]`/`["out"]` for most nodes; logic nodes declare named output pins (`true`/`false`, one per `--case`, `item`) |
| `spec.nodes[].config` | the node's config keys — each DSL flag maps to one, named on the node's page (`--credential` → `credential_id`, `--on-conflict` → `on_conflict`); the `-- "body"` maps to the node's main text (`query` on a query node, `source` on a script node) |
| `spec.edges[]` | `from_node:from_pin → to_node:to_pin`; `from_pin` names a pin (`out`, `true`, a case). Any node's failure may also be routed from the pin `error`. |

`pipeline_get` returns this document; `pipeline_describe` renders it back as DSL
with the node ids, which is what `pipeline_patch node_id=` wants.

---

## Lifecycle

| Status | Meaning |
|---|---|
| `draft` | registered, never activated — serves nothing |
| `active` | live, and the live snapshot equals the file |
| `stale` | live, but the file changed since activation (a re-register or a patch) — traffic still runs the old snapshot until `pipeline_activate` |

`pipeline_deactivate` returns a pipeline to `draft` and keeps the file.
Activation checks the graph before writing the live snapshot — required node
config present, every node kind available (built-in or an installed bundle),
every `zeb/*` library it needs enabled — and refuses with the reason. It does
not compile templates: a wrong `--template` path only shows at request time,
which is why you fetch the route after activating.

---

## Ingress

A webhook pipeline serves at `{METHOD} /wh/{owner}/{project}{--route}`:

```
trigger.webhook --route /api/login --method POST   →   POST /wh/acme/shop/api/login
```

The same route answers a browser (HTML or redirect) and a `fetch` (JSON) —
`web.response.send` decides by what it is given, and auth failures follow the
request kind (303 to the credential's `auth_redirect` for navigations, 401/403
JSON otherwise). Clients that send `Accept: text/event-stream` get the run as
an SSE stream instead of one response.

---

## What a webhook delivers

The trigger answers under its one source key, `webhook`; user data is under
`input.webhook.body`, never at the root:

| Content-Type | `input.webhook.body` |
|---|---|
| `application/json` | the parsed value (object, array, …) |
| `application/x-www-form-urlencoded` | `{ field: value }`, percent-decoded |
| `multipart/form-data` | text fields as `{ field: value }`; files under `input.webhook.files.<field>` as FileRef objects |
| GET, or no body | `null` |

Beside it, in the same `webhook` object: `input.webhook.params` (path
parameters), `input.webhook.query`, `input.webhook.path`,
`input.webhook.method`; and `input.webhook.auth` when `--auth` verified a
token. Repeated fields, `field[]` and `field[0]` become arrays, so a
multi-upload is `input.webhook.files.photos[0]`. `$trigger` is this same
envelope anywhere later in the chain, so the preferred reach past the first
node is `$trigger.body`, `$trigger.params`, `$trigger.query`, `$trigger.auth`.

A FileRef:

```json
{ "__zf_type": "file_ref", "backend": "zebfs", "ref": "tmp/runs/<request_id>/files/<uuid>.jpg",
  "filename": "photo.jpg", "mime": "image/jpeg", "kind": "image", "size": 12345,
  "sha256": "sha256:<64 hex>", "lifecycle": "temporary", "origin": "webhook", "trust": "untrusted" }
```

It is temporary until a node keeps it — `fs.file.put --from "{{ $trigger.files.photo }}"`
writes it into the project's files and answers `file`, the durable FileRef. Bytes never travel inline in the
payload.

---

## Flags and bodies

A node's flags are declared in its definition and the parser refuses any it
does not know, so `help("pipeline/nodes/<kind>")` is the reference. The
grammar every node follows — `--from` for the subject, typed roles, repeated
`key=value` maps, bare switches, units in values, closed choices — is
`help("pipeline/dsl")`. Three things to remember while writing:

- **Statements in the body, values in `--param`**: `sekejap.query.run --param "1={{ $trigger.body.id }}" -- "SELECT … WHERE id = $1"`; `sqlite.query.run` binds `?1`. A write needs `--write`.
- **Code in the body**: `javascript.script.run -- "return { ok: true }"`. `input` and `ctx` are in scope; what it returns is added as `script` (`input.script.ok` downstream) and the rest of the payload is kept.
- **Any value with `{{ }}` or a space is one quoted argument.** A whole-value expression keeps its JSON type; an interpolated one is stringified.
