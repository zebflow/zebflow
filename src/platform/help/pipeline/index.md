# Pipelines

A pipeline is a function: a trigger starts a run, a chain of nodes adds to
the payload, the last node answers. Each node runs once, when every edge into
it has delivered or been skipped (`help("pipeline/dsl")`, Control flow).

```
| trigger.webhook --route /posts/:slug --method GET
| sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT * FROM posts WHERE slug = $1"
| web.response.send --template pages/post.tsx
```

Every node receives the payload as **`input`**, adds **one key** named for
what it made — its noun — and passes the rest on. Above, the trigger adds
`webhook`, the query adds `query` (the rows are `input.query.rows`), and
`web.response.send` renders the page with that payload as the page's `input`.
A node is written as its kind, `family.noun.verb`: `sekejap.query.run`,
`fs.file.put`, `javascript.script.run`. The grammar is `help("pipeline/dsl")`.

## The two envelopes

| | What it is | Where |
|---|---|---|
| **`input`** | The payload flowing along the edges. Each node adds its key to it. | `input` in scripts; `input`/`$input` in `{{ }}` |
| **request context** | The triggering event, frozen at entry: its whole envelope — body, files, path params, query, headers, verified identity. No node changes it. | `ctx.trigger.*` in scripts; `$trigger.*` in `{{ }}` |

### What a webhook puts in `input`

The trigger adds one key, **`webhook`**, holding the whole request; nothing
is merged to the root.

| Key | Content |
|---|---|
| `input.webhook.body` | the JSON body; the fields of a form (urlencoded or multipart); `null` on GET |
| `input.webhook.files.<field>` | uploaded files, as FileRefs |
| `input.webhook.params` | path parameters: `/posts/:slug` → `input.webhook.params.slug` |
| `input.webhook.query` | the parsed query string |
| `input.webhook.path`, `input.webhook.method` | the request line |
| `input.webhook.auth` | the verified token's claims, when the trigger has `--auth` |

A login form is `input.webhook.body.email` right after the trigger. `$trigger`
is the same envelope for the whole run, so anywhere later the fields read
`$trigger.body.email`, `$trigger.params`, `$trigger.query`, `$trigger.auth`,
`$trigger.headers` — plus `$trigger.search` and `$trigger.pathname`, which a
webhook adds.

### JWT on a route

```
| trigger.webhook --route /admin --method GET --auth jwt --credential jwt_main --role admin
```

The token comes from `Authorization: Bearer …` or, for browsers, the
`zebflow_session` cookie; its `roles` **array** must contain one of the
required roles. A browser that fails is redirected (303) to the credential's
`auth_redirect`; a `fetch` gets 401/403 JSON. Claims minted with `:public`
reach a page as `input.auth`. Recipe: `help("pipeline/examples/cookie-jwt-auth")`.

---

## Two modes

**Pipe mode** — a straight chain, one node per `|`:

```
| trigger.webhook --route /api/notes --method GET
| sekejap.query.run -- "SELECT id, title FROM notes ORDER BY created_at DESC LIMIT 50"
| web.response.send --body "{{ input.query.rows }}"
```

**Graph mode** — `[id]` labels, `->` edges, `:pin` for branches; for
branching, fan-out, joins and loops:

```
[a] trigger.webhook --route /api/notes --method POST
[b] logic.if --when "typeof input.webhook.body?.title === 'string' && input.webhook.body.title.length > 0"
[c] sekejap.query.run --write --param "1={{ $trigger.body.title }}" -- "INSERT INTO notes (title) VALUES ($1)"
[d] web.response.send --status 201 --body "{{ { ok: true } }}"
[e] web.response.send --status 400 --body "{{ { error: 'title is required' } }}"
[a] -> [b]
[b]:true -> [c]
[c] -> [d]
[b]:false -> [e]
```

Every node must be reachable from the one entry; a node with no incoming
edge is a second entry and runs on every request. A node runs once, when
every edge into it has delivered or been skipped: above, `d` and `e` are on
different branches, and the one not taken is skipped. A branch meeting
another is a join, not a second run; a cycle is refused unless it is a
`logic.retry` going round again.

---

## Finding the node you need

| Call | What you get |
|---|---|
| `help("pipeline/nodes")` | every kind on one line, by family — find the name first |
| `help("pipeline/nodes/fs.file.put")` | one node: its signature, what it answers, every flag, examples; one signature per `--provider` |
| `help_search query="thumbnail"` | the help **and** every node's description and flags |
| `help("pipeline/nodes/all")` | the whole catalogue (large) |

A node accepts only the flags it declares; an undeclared flag is a parse
error. Read the node's page before the first use in a session.

---

## Registering and activating

A pipeline is identified by its **`file_rel_path`** — the `.zf.json` path
inside the project's source root (the repository root unless `zebflow.yaml`
sets `spec.layout.source`). The extension may be omitted:

```
api/posts        →  api/posts.zf.json
pages/blog-home  →  pages/blog-home.zf.json
```

**Register** saves a draft; **activate** promotes it to live traffic:

```
pipeline_register  file_rel_path="api/posts"  title="Posts"  body="| trigger.webhook --route /api/posts --method GET | sekejap.query.run -- \"SELECT * FROM posts\" | web.response.send --body \"{{ input.query.rows }}\""
pipeline_activate  file_rel_path="api/posts"
```

Status is **`active`** (live and current), **`stale`** (live, but changed
since activation — run `pipeline_activate`) or **`draft`** (never
activated). `pipeline_get_invocations` shows what a live pipeline did.

To change one node: `pipeline_describe` (node ids `n0, n1, …`) →
`pipeline_patch node_id="n1" flags="--limit 100"` → `pipeline_activate`. To
try a body without saving: `pipeline_run body="| trigger.function | javascript.script.run -- \"return 1\""`
(`input` gives it a payload).

---

## Common web patterns

**A page from the database**

```
| trigger.webhook --route /blog --method GET
| sekejap.query.run -- "SELECT id, title, slug, created_at FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response.send --template pages/blog-home.tsx
```

**A route for signed-in users**

```
| trigger.webhook --route /dashboard --method GET --auth jwt --credential jwt_main
| sekejap.query.run --param "1={{ $trigger.auth.sub }}" -- "SELECT id, name FROM users WHERE id = $1"
| web.response.send --template pages/dashboard.tsx
```

**A redirect**

```
| trigger.webhook --route /go/signup --method GET
| web.response.send --status 302 --header "Location=/auth/register?source=landing"
```

**A scheduled job**

```
| trigger.schedule --cron "0 * * * *" --timezone UTC
| http.response.fetch --url https://api.example.com/feed --method GET
| javascript.script.run -- "return { items: (input.response.body?.items || []).slice(0, 10) }"
| kv.entry.put --key feed:latest --value "{{ input.script.items }}" --ttl 1h
```

A script's return is added as `script` and the rest of the payload is kept.
A script cannot set a status or a header: branch with `logic.if` and answer
with `web.response.send --status`, `--header` or `--body`.

---

## `{{ expr }}`

Any flag value may contain `{{ js_expression }}`, resolved right before the
node runs. A value that is only an expression keeps its JSON type; one
inside a longer string is stringified.

| Name | Meaning |
|---|---|
| `input`, `$input` | the payload arriving at this node |
| `$trigger` | the trigger's envelope for the whole run |
| `$nodes.<id>` | an upstream node's payload; its answer is `$nodes.<id>.<key>` — `null` when it was skipped |
| `$item`, `$index`, `$count` | inside `logic.foreach` |

Always quote a value that contains `{{ }}` or a space as one argument.

Full grammar: `help("pipeline/dsl")`. The JSON model: `help("pipeline/authoring")`.
Responses, cookies, redirects: `help("pipeline/web")`. Pages: `help("web")`.
Complete recipes: `help("pipeline/examples")`.
