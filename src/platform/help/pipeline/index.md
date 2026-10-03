# Pipelines

A pipeline is a function: a trigger produces an input, a chain of nodes
transforms it, the last node answers.

```
| trigger.webhook --route /posts/:slug --method GET
| sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response.send --template pages/post.tsx
```

Every node receives the previous node's output as **`input`** and returns the
next payload. `sekejap.query.run` replaces it with `{ query: { columns, rows, … } }`; a
`script` shapes it; `web.response.send` turns it into an HTTP response or a page.
Nothing flows unless a node passes it on.

## The two envelopes

| | What it is | Where |
|---|---|---|
| **`input`** | The business payload flowing along the edges. Each node transforms it. | `input` in scripts; `input`/`$input` in `{{ }}` |
| **request context** | The triggering event, frozen at entry: its whole envelope — body/files, path params, query, headers, verified identity. Never changed by any node. | `ctx.trigger.*` in scripts; `$trigger.*` in `{{ }}` |

So when `sekejap.query.run` has replaced the payload, the caller's identity is
still `ctx.trigger.auth.sub` in a script and `$trigger.auth.sub` in a flag.
`ctx.request_id` and `ctx.pipeline` are there too.

### What a webhook puts in `input`

The trigger adds exactly one key, **`webhook`**, holding the whole request
envelope. Nothing is merged to the root.

| Key | Content |
|---|---|
| `input.webhook.body` | JSON body as parsed; form fields for `application/x-www-form-urlencoded`; text fields of a multipart form. `null` on GET. |
| `input.webhook.files.<field>` | Uploaded files as FileRef objects (`ref`, `filename`, `mime`, `size`, `sha256`, …) |
| `input.webhook.params` | Path parameters: `/posts/:slug` → `input.webhook.params.slug` |
| `input.webhook.query` | Parsed query string |
| `input.webhook.path`, `input.webhook.method` | The request line |
| `input.webhook.auth` | Verified token claims, when the trigger has `--auth` |

A login form is therefore `input.webhook.body.email` and
`input.webhook.body.password` right after the trigger. `$trigger` is this
same envelope for the whole run, so deeper in a chain the same fields read as
`$trigger.body.email`, `$trigger.params`, `$trigger.query`, `$trigger.auth`,
`$trigger.headers` — plus `$trigger.search` and `$trigger.pathname`, which
only a webhook trigger adds.

### JWT on a route

```
| trigger.webhook --route /admin --method GET --auth jwt --credential jwt_main --role admin
```

The token comes from `Authorization: Bearer …` or, for browsers, the
`zebflow_session` cookie. Its `roles` **array** claim must contain one of the
required roles. A browser navigation that fails is redirected (303) to the
credential's `auth_redirect`; a `fetch` gets 401/403 JSON. Verified claims
appear as `input.webhook.auth` right after the trigger, or `ctx.trigger.auth`
and `$trigger.auth` anywhere in the run; only claims minted with `:public`
reach the browser as `input.auth` in a page.
Full recipe: `help(topic="pipeline/examples/cookie-jwt-auth")`.

---

## Two modes

**Pipe mode** — a straight chain, one node per `|`. Most pipelines.

```
| trigger.webhook --route /api/notes --method GET
| sekejap.query.run -- "SELECT id, title FROM notes ORDER BY created_at DESC LIMIT 50"
| script.result.run -- "return { notes: input.query.rows }"
```

**Graph mode** — label nodes `[id]`, wire edges with `->`, name pins with `:pin`.
For branching, fan-out and loops.

```
[a] trigger.webhook --route /ingest --method POST
[b] logic.match --expr "input.webhook.body.type" --cases normal,urgent --default other
[c] sekejap.query.run --param "1={{ $trigger.body.id }}" --param "2={{ $trigger.body }}" --write -- "INSERT INTO normal_queue (id, data) VALUES ($1, $2)"
[d] http.response.fetch --url https://alert.example.com/send --method POST --body "{{ $trigger.body }}"
[e] sekejap.query.run --param "1={{ $trigger.body.id }}" --param "2={{ $trigger.body }}" --write -- "INSERT INTO other_queue (id, data) VALUES ($1, $2)"
[a] -> [b]
[b]:normal -> [c]
[b]:urgent -> [d]
[b]:other -> [e]
```

Every node must be reachable from the one entry node; a node with no incoming
edge is a second entry and runs on every request.

---

## Nodes

- **Triggers** start a run: `trigger.webhook`, `trigger.schedule`, `trigger.function`, `trigger.manual`, `trigger.room`, `trigger.socket`, `trigger.topic`, `trigger.mcp`, `trigger.error`.
- **Middle nodes** read, transform or decide: `sekejap.query.run`, `sekejap.record.create`, `pg.query.run`, `sqlite.query.run`, `script`, `http.response.fetch`, `kv.entry.get`, `kv.entry.put`, `kv.entry.increment`, `logic.if`, `logic.match`, `logic.foreach`, `logic.collect`, `logic.reduce`, `logic.retry`, `crypto.*`, `auth.token.create`, `auth.token.verify`, `fs.file.put`, `fs.image.thumbnail`, `fs.*`, `table.query.run`, `table.data.convert`, `geo.*`, `mail.message.send`, `ai.text.generate`, `ai.embedding.generate`, `ai.audio.generate`, `browser.page.run`, …
- **Last nodes** answer: `web.response.send` (JSON, page, redirect, cookie — `help(topic="pipeline/web")`), or push: `ws.message.send`, `ws.state.update`, `kv.message.publish`, `telegram.send`, `ms.layer.publish`.

Flags are declared per node and an undeclared flag is a parse error, so read
the node before guessing:

| Call | What you get |
|---|---|
| `help(topic="pipeline/nodes")` | the index: every kind on one line, by family — find the name first |
| `help(topic="pipeline/nodes/all")` | the whole catalogue with every flag table and schema (large) |
| `help(topic="pipeline/nodes/fs.file.put")` | one node: description, pins, every flag with its config key, required or not |
| `help_search query="thumbnail"` | search across the help files **and** every node's description and flags |

The DSL accepts the short form (`trigger.webhook`, `sekejap.query.run`) or the full
kind (`trigger.webhook`). Installed third-party nodes are `n.x.<bundle>.<node>`.

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
pipeline_register  file_rel_path="api/posts"  title="Posts"  body="| trigger.webhook --route /api/posts --method GET | sekejap.query.run -- \"SELECT * FROM posts\""
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

To try a body without saving anything: `pipeline_run body="| trigger.function | script.result.run -- \"return 1\""`
(`input` gives it a payload).

---

## Common web patterns

**GET page from the database**

```
| trigger.webhook --route /blog --method GET
| sekejap.query.run -- "SELECT id, title, slug, created_at FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response.send --template pages/blog-home.tsx
```

**POST JSON API — validate, insert, answer**

```
[a] trigger.webhook --route /api/posts --method POST
[b] logic.if --expr "typeof input.webhook.body?.title === 'string' && input.webhook.body.title.length > 0"
[c] sekejap.query.run --param "1={{ $trigger.body.title }}" --param "2={{ $trigger.body.title.toLowerCase().replace(/\s+/g, '-') }}" --write -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
[d] script.result.run -- "return { ok: true }"
[e] web.response.send --status 400 --body "{{ { error: 'title is required' } }}"
[a] -> [b]
[b]:true -> [c]
[c] -> [d]
[b]:false -> [e]
```

**Authenticated route**

```
| trigger.webhook --route /dashboard --method GET --auth jwt --credential jwt_main
| sekejap.query.run --param "1={{ $trigger.auth.sub }}" -- "SELECT id, name FROM users WHERE id = $1"
| web.response.send --template pages/dashboard.tsx
```

**Redirect**

```
| trigger.webhook --route /go/signup --method GET
| web.response.send --status 302 --header "Location=/auth/register?source=landing"
```

**Scheduled job**

```
| trigger.schedule --cron "0 * * * *" --timezone UTC
| http.response.fetch --url https://api.example.com/feed --method GET
| script.result.run -- "return { items: (input.response.body?.items || []).slice(0, 10) }"
| kv.entry.put --key feed:latest --ttl 3600
```

A script cannot set the HTTP status or headers; it returns the next payload.
Branch with `logic.if` and let `web.response.send` answer with `--status`,
`--header` or `--body`. Returning `null` from a script does not stop
the pipeline either — `null` simply becomes the next `input`.

---

## `{{ expr }}` — dynamic config

Any flag value may contain `{{ js_expression }}`, resolved right before the
node runs. A value that is **only** an expression keeps its JSON type
(`"{{ [input.id] }}"` is a real array); an expression inside a longer
string is stringified.

| Name | Meaning |
|---|---|
| `input`, `$input` | the payload flowing into this node |
| `$trigger` | the trigger's envelope for the whole run — for a webhook: `body`, `query`, `params`, `headers`, `files`, `method`, `path`, `auth`, plus `search` and `pathname` |
| `$nodes.<id>` | the output of an upstream node by graph id |
| `$item`, `$index`, `$count` | inside `logic.foreach` |

There is no `ctx`, `$ctx` or `env` in `{{ }}`; an undefined name throws and
fails the node rather than silently inserting `null`. Always quote a value
that contains `{{ }}` or a space as one argument.

Full DSL: `help(topic="pipeline/dsl")`. Pages: `help(topic="web")`. Responses,
cookies, redirects: `help(topic="pipeline/web")`. Complete recipes:
`help(topic="pipeline/examples")`.
