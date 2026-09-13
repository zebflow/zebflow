# Pipelines

A pipeline is a function: a trigger produces an input, a chain of nodes
transforms it, the last node answers.

```
| trigger.webhook --path /posts/:slug --method GET
| sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response --template pages/post.tsx
```

Every node receives the previous node's output as **`input`** and returns the
next payload. `sekejap.query` replaces it with `{ columns, rows, … }`; a
`script` shapes it; `web.response` turns it into an HTTP response or a page.
Nothing flows unless a node passes it on.

## The two envelopes

| | What it is | Where |
|---|---|---|
| **`input`** | The business payload flowing along the edges. Each node transforms it. | `input` in scripts; `input`/`$input` in `{{ }}` |
| **request context** | The triggering event, frozen at entry: path params, query, headers, verified identity. Never changed by any node. | `ctx.trigger.*` in scripts; `$trigger.*` in `{{ }}` |

So when `sekejap.query` has replaced the payload, the caller's identity is
still `ctx.trigger.auth.sub` in a script and `$trigger.auth.sub` in a flag.
`ctx.request_id` and `ctx.pipeline` are there too.

### What a webhook puts in `input`

User-submitted data lives under **`input.body`**; request context sits beside
it. Nothing is merged to the root.

| Key | Content |
|---|---|
| `input.body` | JSON body as parsed; form fields for `application/x-www-form-urlencoded`; text fields of a multipart form. `null` on GET. |
| `input.files.<field>` | Uploaded files as FileRef objects (`ref`, `filename`, `mime`, `size`, `sha256`, …) |
| `input.params` | Path parameters: `/posts/:slug` → `input.params.slug` |
| `input.query` | Parsed query string |
| `input.path`, `input.method` | The request line |
| `input.auth` | Verified token claims, when the trigger has `--auth-type` |

A login form is therefore `input.body.email` and `input.body.password`.
In `{{ }}` the same request is `$trigger.params`, `$trigger.query`,
`$trigger.auth`, `$trigger.headers`, `$trigger.search`, `$trigger.pathname`
— `$trigger` has no `body`; reach the body as `{{ input.body.email }}` while
`input` is still the trigger payload, or in a script.

### JWT on a route

```
| trigger.webhook --path /admin --method GET --auth-type jwt --auth-credential jwt_main --auth-required-role admin
```

The token comes from `Authorization: Bearer …` or, for browsers, the
`zebflow_session` cookie. Its `roles` **array** claim must contain one of the
required roles. A browser navigation that fails is redirected (303) to the
credential's `auth_redirect`; a `fetch` gets 401/403 JSON. Verified claims
appear as `input.auth`, `ctx.trigger.auth` and `$trigger.auth`; only claims
minted with `:public` reach the browser as `input.auth` in a page.
Full recipe: `help(topic="pipeline/examples/cookie-jwt-auth")`.

---

## Two modes

**Pipe mode** — a straight chain, one node per `|`. Most pipelines.

```
| trigger.webhook --path /api/notes --method GET
| sekejap.query -- "SELECT id, title FROM notes ORDER BY created_at DESC LIMIT 50"
| script -- "return { notes: input.rows }"
```

**Graph mode** — label nodes `[id]`, wire edges with `->`, name pins with `:pin`.
For branching, fan-out and loops.

```
[a] trigger.webhook --path /ingest --method POST
[b] logic.match --expr "input.body.type" --cases normal,urgent --default other
[c] sekejap.query --params "{{ [input.body.id, input.body] }}" --read-only false -- "INSERT INTO normal_queue (id, data) VALUES ($1, $2)"
[d] http.request --url https://alert.example.com/send --method POST --body "{{ input.body }}"
[e] sekejap.query --params "{{ [input.body.id, input.body] }}" --read-only false -- "INSERT INTO other_queue (id, data) VALUES ($1, $2)"
[a] -> [b]
[b]:normal -> [c]
[b]:urgent -> [d]
[b]:other -> [e]
```

Every node must be reachable from the one entry node; a node with no incoming
edge is a second entry and runs on every request.

---

## Nodes

- **Triggers** start a run: `trigger.webhook`, `trigger.schedule`, `trigger.function`, `trigger.manual`, `trigger.ws`, `trigger.ws.client`, `trigger.kv.subscribe`, `trigger.mcp`, `trigger.weberror`.
- **Middle nodes** read, transform or decide: `sekejap.query`, `sekejap.insert`, `pg.query`, `sqlite.query`, `sqlite.mutate`, `script`, `http.request`, `kv.get`, `kv.set`, `kv.incr`, `logic.if`, `logic.match`, `logic.foreach`, `logic.collect`, `logic.reduce`, `logic.retry`, `crypto`, `auth.token.create`, `auth.token.verify`, `fs.save`, `fs.thumbnail`, `fs.*`, `table.query`, `table.convert`, `geo.*`, `mail.send`, `ai.agent`, `ai.embedding`, `ai.tts`, `browser.run`, …
- **Last nodes** answer: `web.response` (JSON, page, redirect, cookie — `help(topic="pipeline/web")`), or push: `ws.emit`, `ws.sync_state`, `kv.publish`, `telegram.send`, `ms.publish`.

Flags are declared per node and an undeclared flag is a parse error, so read
the node before guessing:

| Call | What you get |
|---|---|
| `help(topic="pipeline/nodes")` | the whole catalog, generated from the node definitions |
| `help(topic="pipeline/nodes/n.fs.save")` | one node: description, pins, every flag with its config key, required or not |
| `help_search query="thumbnail"` | search across the help files **and** every node's description and flags |

The DSL accepts the short form (`trigger.webhook`, `sekejap.query`) or the full
kind (`n.trigger.webhook`). Installed third-party nodes are `n.x.<bundle>.<node>`.

---

## Registering and activating

A pipeline is identified by its **`file_rel_path`** — the `.zf.json` path
inside the project's source root. The source root is the repository root
unless `zebflow.yaml` sets `spec.layout.source`; it is never part of the
identifier. The extension may be omitted:

```
api/posts        →  api/posts.zf.json
pages/blog-home  →  pages/blog-home.zf.json
```

**Register** saves a draft; **activate** promotes it to live traffic.

```
pipeline_register  file_rel_path="api/posts"  title="Posts"  body="| trigger.webhook --path /api/posts --method GET | sekejap.query -- \"SELECT * FROM posts\""
pipeline_activate  file_rel_path="api/posts"
```

Or in the project console: `register api/posts --title "Posts" | trigger.webhook … | …`
then `activate pipeline api/posts`.

Status is one of **`active`** (live and current), **`stale`** (live, but the
file changed since activation — re-registering or patching does not promote;
run `pipeline_activate`), **`draft`** (never activated). `pipeline_list`
shows it; `pipeline_get_invocations` shows what a live pipeline actually did.

To change one node without rewriting the graph:

```
pipeline_describe  file_rel_path="api/posts"                      ← node ids: n0, n1, …
pipeline_patch     file_rel_path="api/posts"  node_id="n1"  flags="--limit 100"
pipeline_activate  file_rel_path="api/posts"
```

To try a body without saving anything: `pipeline_run body="| trigger.function | script -- \"return 1\""`
(`input` gives it a payload).

---

## Common web patterns

**GET page from the database**

```
| trigger.webhook --path /blog --method GET
| sekejap.query -- "SELECT id, title, slug, created_at FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx
```

**POST JSON API — validate, insert, answer**

```
[a] trigger.webhook --path /api/posts --method POST
[b] logic.if --expr "typeof input.body?.title === 'string' && input.body.title.length > 0"
[c] sekejap.query --params "{{ [input.body.title, input.body.title.toLowerCase().replace(/\s+/g, '-')] }}" --read-only false -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
[d] script -- "return { ok: true }"
[e] web.response --status 400 --body "{{ { error: 'title is required' } }}"
[a] -> [b]
[b]:true -> [c]
[c] -> [d]
[b]:false -> [e]
```

**Authenticated route**

```
| trigger.webhook --path /dashboard --method GET --auth-type jwt --auth-credential jwt_main
| sekejap.query --params "{{ [$trigger.auth.sub] }}" -- "SELECT id, name FROM users WHERE id = $1"
| web.response --template pages/dashboard.tsx
```

**Redirect**

```
| trigger.webhook --path /go/signup --method GET
| web.response --location "/auth/register?source=landing"
```

**Scheduled job**

```
| trigger.schedule --cron "0 * * * *" --timezone UTC
| http.request --url https://api.example.com/feed --method GET
| script -- "return { items: (input.response.body?.items || []).slice(0, 10) }"
| kv.set --key feed:latest --ttl 3600
```

A script cannot set the HTTP status or headers; it returns the next payload.
Branch with `logic.if` and let `web.response` answer with `--status`,
`--location` or `--set-cookie`. Returning `null` from a script does not stop
the pipeline either — `null` simply becomes the next `input`.

---

## `{{ expr }}` — dynamic config

Any flag value may contain `{{ js_expression }}`, resolved right before the
node runs. A value that is **only** an expression keeps its JSON type
(`"{{ [input.body.id] }}"` is a real array); an expression inside a longer
string is stringified.

| Name | Meaning |
|---|---|
| `input`, `$input` | the payload flowing into this node |
| `$trigger` | the trigger snapshot: `params`, `query`, `search`, `pathname`, `headers`, `auth` |
| `$nodes.<id>` | the output of an upstream node by graph id |
| `$item`, `$index`, `$count` | inside `logic.foreach` |

There is no `ctx`, `$ctx` or `env` in `{{ }}`; an undefined name throws and
fails the node rather than silently inserting `null`. Always quote a value
that contains `{{ }}` or a space as one argument.

Full DSL: `help(topic="pipeline/dsl")`. Pages: `help(topic="web")`. Responses,
cookies, redirects: `help(topic="pipeline/web")`. Complete recipes:
`help(topic="pipeline/examples")`.
