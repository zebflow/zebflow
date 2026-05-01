# Pipeline System Guide

## Mental Model

**A pipeline is a function.**

```
pipeline(trigger_input) → response
```

Inside that function there are three distinct things:

| Concept | What it is | Accessible via |
|---------|-----------|----------------|
| **Run context** | Immutable snapshot of the triggering event — who called, from where, verified identity. Set once at entry, available to every node unchanged. | `ctx.*` in script nodes |
| **Nodes** | Individual operations — query, transform, decide, render. Each is a pure function: takes `input`, returns the next payload. | declared in graph |
| **Graph** | The data flow — which node's output becomes which node's input. Explicit dependencies. Nothing flows unless you wire it. | edges (`->`) |

### The two envelopes

Every node receives **two separate things**:

- **`input`** — the business payload flowing through edges. Each node transforms it. `pg.query` replaces it with `{rows:[...]}`. A `script` node shapes it. This is the graph's concern.
- **`ctx`** — the run-level context. Same on every node. Never touched by edges. Contains `ctx.pipeline`, `ctx.request_id`, and for webhook-triggered pipelines: `ctx.trigger.auth`, `ctx.trigger.params`, `ctx.trigger.query`, `ctx.trigger.headers`.

The mental model maps directly to code:

```js
// pipeline execution is essentially:
function myPipeline(trigger_input, ctx) {
  const a = nodeA(trigger_input, ctx);   // e.g. pg.query
  const b = nodeB(a, ctx);              // e.g. script transform
  const c = nodeC(b, ctx);              // e.g. web.response
  return c;
}
// ctx is always available — it never flows through edges
// input is whatever the upstream node returned — it can be anything
```

### Why this matters

- **Lost auth problem**: if `pg.query` replaces the payload with `{rows:[...]}`, auth is gone from `input.auth` — but `ctx.trigger.auth` is never lost. Use `ctx.trigger.auth` in script nodes when you need caller identity regardless of where you are in the graph.
- **Graph stays clean**: edges carry business data only. Auth, params, request identity are ambient — they don't pollute the data flow.
- **`web.response` + templates**: `ctx.trigger.auth` public claims are always injected as `ctx.auth` in the template, same as `ctx.params` and `ctx.query` — regardless of what the pipeline chain did to the payload.

### `ctx.trigger.*` reference

Available in script nodes (`n.script`) as `ctx.trigger.*` and in templates as top-level state fields:

| Field | Script node | Template | Description |
|-------|-------------|----------|-------------|
| `ctx.trigger.auth` | `ctx.trigger.auth.sub` | `ctx.auth.sub` | Verified JWT claims (public fields only in templates) |
| `ctx.trigger.params` | `ctx.trigger.params.id` | `ctx.params.id` | URL path params from the request (`:id`, `:slug`, etc.) |
| `ctx.trigger.query` | `ctx.trigger.query.page` | `ctx.query.page` | Query string params (`?page=2` etc.) |
| `ctx.trigger.headers` | `ctx.trigger.headers["user-agent"]` | `ctx.headers["user-agent"]` | Safe subset of request headers |

All fields are `null` / empty objects for non-webhook triggers (schedule, WS, manual).

### JWT auth — Bearer + Cookie fallback

When `--auth-type jwt` is set, the webhook checks:
1. `Authorization: Bearer <token>` header
2. `Cookie: zebflow_session` (fallback — used by browser sessions)

Both paths verify with the same credential. If neither is present, auth fails.

### `_zf_public` — what reaches `ctx.auth` in templates

JWT claims are **private by default**. Only claims explicitly marked `:public` in `auth.token.create` are visible in the browser via `ctx.auth`:

```zf
| auth.token.create --credential my-jwt \
    --claim sub=$.id \
    --claim name=$.fullname:public \   ← visible as ctx.auth.name
    --claim role=$.role:public         ← visible as ctx.auth.role
    # sub is signed but never reaches the browser
```

If no claims are marked `:public`, `ctx.auth` is `null` even for authenticated requests — secure by default.

In **script node `{{ expr }}` expressions**, `$trigger.auth` holds **all** claims (including private). The `_zf_public` filter only applies at the `n.web.response` render boundary.

---

When you call **`help(topic="pipeline")`** over MCP, the platform **appends a live node appendix** after this file: every node `kind`, description, pins, DSL flags, and input/output schemas come from the same Rust `definition()` as the pipeline editor node API (not from hand-written docs).

---

## Two Modes

### Pipe Mode (most common — use this)

Use `|` to chain nodes left-to-right. Every node receives the previous node's output as `input`.

```zf
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, slug FROM posts ORDER BY created_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx --route /blog
```

Pass this as the `body` to `pipeline_register`.

### Graph Mode (for branching)

Label each node with `[id]`, then declare edges with `->`. Use for conditional routing, fan-out, loops.

```zf
[a] trigger.webhook --path /ingest --method POST
[b] logic.match --expr "input.type" --cases normal,urgent --default other
[c] sekejap.query --table normal_queue --op upsert
[d] http.request --url https://alert.svc/send --method POST
[e] sekejap.query --table other_queue --op upsert
[a] -> [b]
[b]:normal -> [c]
[b]:urgent -> [d]
[b]:other -> [e]
```

---

## Nodes

A **node** is one step in the pipeline. Each node has a **kind** (trigger, query, render, logic, …). The previous node’s JSON output becomes the next node’s **`input`**.

- **Triggers** start a run (`trigger.webhook`, `trigger.schedule`, `trigger.ws`, `trigger.memsubscribe`, …).
- **Middle nodes** read or transform data (`pg.query`, `sekejap.query`, `script`, `http.request`, `mem.get`, `mem.set`, `mem.incr`, `logic.if`, …).
- **Last nodes** produce the HTTP response (`web.response` for HTML pages, JSON, redirects, or cookies — see `help(topic="pipeline/web")`), or push real-time updates (`ws.emit`, `ws.sync_state`, `mem.publish`).

### How to open the full node reference

Use `help(topic="pipeline/nodes")` instead of guessing flags:

| Call | What you get |
|------|----------------|
| `help(topic="pipeline/nodes")` | The **entire** node catalog (same generated appendix as `help(topic="pipeline")`). |
| `help(topic="pipeline/nodes/{kind}")` | **One** node only — e.g. `topic="pipeline/nodes/n.trigger.webhook"`, `topic="pipeline/nodes/n.script"`. |

Use **`help_search`** for keywords across remaining skill markdown (node bodies are not duplicated there — use `help(topic="pipeline/nodes/…")` for node flags and schemas).

---

## Registering and Activating

### The Pipeline Identifier: `file_rel_path`

Every pipeline is identified by its `file_rel_path` — the path of its `.zf.json` file under the project repo.

```
pipelines/pages/blog-home.zf.json   → file_rel_path = "pipelines/pages/blog-home.zf.json"
pipelines/api/posts.zf.json         → file_rel_path = "pipelines/api/posts.zf.json"
pipelines/my-pipe.zf.json           → file_rel_path = "pipelines/my-pipe.zf.json"
```

You can omit the `pipelines/` prefix and `.zf.json` extension — they are added automatically:
```
pipelines/pages/blog-home  →  pipelines/pages/blog-home.zf.json
pages/blog-home            →  pipelines/pages/blog-home.zf.json
blog-home                  →  pipelines/blog-home.zf.json
```

**Never use `name` or `path` as separate pipeline parameters.** `file_rel_path` is the only key.

### Step 1: Register (saves as draft)

```
pipeline_register
  file_rel_path = "pipelines/pages/blog-home"
  title = "Blog Home"
  body = "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\" | web.response --template pages/blog-home.tsx --route /blog"
```

### Step 2: Activate (goes live)

```
pipeline_activate  file_rel_path="pipelines/pages/blog-home.zf.json"
```

Or via DSL shell: `activate pipelines/pages/blog-home.zf.json`

After activate, the pipeline handles live traffic.

### Updating a Pipeline

Option A — re-register with full new body (easiest):
```
pipeline_register  file_rel_path="pipelines/pages/blog-home"  body="..."
pipeline_activate  file_rel_path="pipelines/pages/blog-home.zf.json"
```

Option B — patch one node without rewriting the graph:
```
pipeline_describe  file_rel_path="pipelines/pages/blog-home.zf.json"   ← get node IDs
pipeline_patch     file_rel_path="pipelines/pages/blog-home.zf.json"  node_id="n1"  flags="--credential new-db"
pipeline_activate  file_rel_path="pipelines/pages/blog-home.zf.json"
```

---

## Common Fullstack Web Patterns

### GET Page — render HTML from DB

```
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, body, created_at FROM posts ORDER BY created_at DESC"
| web.response --template pages/blog-home.tsx --route /blog
```

### POST JSON API — insert and return JSON

```
| trigger.webhook --path /api/posts --method POST
| script -- "return { title: input.title, slug: input.title.toLowerCase().replace(/\s+/g,'-'), created_at: Date.now() }"
| sekejap.query --table posts --op upsert
| script -- "return { ok: true, slug: input.slug }"
```

### Auth-Gated Route — JWT auto-verify

```
| trigger.webhook --path /dashboard --method GET --auth-type jwt --auth-credential my-jwt
| pg.query --credential main-db -- "SELECT id, name FROM users WHERE id = $1" --params-expr "[ctx.trigger.auth.sub]"
| web.response --template pages/dashboard.tsx
```

JWT missing/invalid → credential `auth_redirect` fires (browser) or 401 JSON (API). `ctx.trigger.auth` holds the decoded claims in all downstream nodes. Template gets `ctx.auth` automatically.

### Redirect

```
| trigger.webhook --path /go/signup --method GET
| script -- "return { __redirect: '/auth/register?source=landing' }"
```

### Scheduled Job — run every hour

```
| trigger.schedule --cron "0 * * * *"
| http.request --url https://api.example.com/feed --method GET
| script -- "return input.response.body.items.slice(0,10)"
| sekejap.query --table feed_cache --op upsert
```

### Sekejap CRUD — read from embedded database

```
| trigger.webhook --path /api/notes --method GET
| sekejap.query --table notes --op scan
| script -- "return { notes: input }"
```

---

## Dynamic expressions — `{{ expr }}`

Any string field in a node's config can contain `{{ js_expr }}` placeholders resolved before the node runs.

| Variable | Available from | Description |
|----------|---------------|-------------|
| `$input` | All nodes | Current payload flowing into this node |
| `$trigger.auth` | All nodes | Verified JWT claims from the original request |
| `$trigger.params` | All nodes | URL path params (`:id` etc.) |
| `$trigger.query` | All nodes | Query string params |
| `$nodes.id` | All nodes | Output payload of an upstream node by graph ID |
| `$ctx` | All nodes | `{ pipeline, request_id, trigger }` |

```zf
# Path param → SQL param (whole-field expr → native array type)
| pg.query --params-expr "{{ [$trigger.params.id] }}"

# Dynamic URL from upstream output (interpolated expr → string)
| http.request --url "https://api.example.com/{{ $nodes.userQuery.rows[0].slug }}"
```

See `help(topic="pipeline/dsl")` for the full `{{ expr }}` reference including sandbox security guarantees.

---

## Next steps

- `help(topic="pipeline/nodes")` — same live catalog as the appendix on `help(topic="pipeline")`, or one node via `topic="pipeline/nodes/{kind}"`
- `help(topic="pipeline/dsl")` — `{{ expr }}` dynamic expressions, full node DSL flags
- `help(topic="web")` — TSX pages, `input` / `ctx`, hydration modes
- `help(topic="pipeline/web")` — `n.web.response` flags, cookie spec, redirect, custom headers
- `help(topic="pipeline/examples")` — full archetype recipes (blog, chat, game, scheduling, scraping, auth)
