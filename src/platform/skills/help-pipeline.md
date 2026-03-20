# Pipeline System Guide

Pipelines are the core of Zebflow. A pipeline is a linear or branching chain of nodes that handles an HTTP request, WebSocket event, or scheduled trigger — and produces a response (HTML page, JSON, redirect, or side effects).

---

## Two Modes

### Pipe Mode (most common — use this)

Use `|` to chain nodes left-to-right. Every node receives the previous node's output as `input`.

```zf
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, slug FROM posts ORDER BY created_at DESC LIMIT 20"
| web.render --template-path pages/blog-home.tsx --route /blog
```

Pass this as the `body` to `pipeline_register`.

### Graph Mode (for branching)

Label each node with `[id]`, then declare edges with `->`. Use for conditional routing, fan-out, loops.

```zf
[a] trigger.webhook --path /ingest --method POST
[b] logic.switch --expr "input.body.type" --cases normal,urgent --default other
[c] sekejap.query --table normal_queue --op upsert
[d] http.request --url https://alert.svc/send --method POST
[e] sekejap.query --table other_queue --op upsert
[a] -> [b]
[b]:normal -> [c]
[b]:urgent -> [d]
[b]:other -> [e]
```

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
  body = "| trigger.webhook --path /blog --method GET | pg.query --credential main-db -- \"SELECT * FROM posts\" | web.render --template-path pages/blog-home.tsx --route /blog"
```

### Step 2: Activate (goes live)

```
pipeline_activate  file_rel_path="pipelines/pages/blog-home.zf.json"
```

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
| web.render --template-path pages/blog-home.tsx --route /blog
```

### POST JSON API — insert and return JSON

```
| trigger.webhook --path /api/posts --method POST
| script -- "return { title: input.body.title, slug: input.body.title.toLowerCase().replace(/\s+/g,'-'), created_at: Date.now() }"
| sekejap.query --table posts --op upsert
| script -- "return { ok: true, slug: input.slug }"
```

### Auth-Gated Route — check JWT before serving

```
| trigger.webhook --path /dashboard --method GET
| script -- "const tok = input.headers['authorization']?.replace('Bearer ',''); if (!tok) return { __redirect: '/login' }; return input;"
| pg.query --credential main-db -- "SELECT id, name, role FROM users WHERE id = '{{input.user_id}}'"
| web.render --template-path pages/dashboard.tsx --route /dashboard
```

### Redirect

```
| trigger.webhook --path /go/signup --method GET
| script -- "return { __redirect: '/auth/register?source=landing' }"
```

### Scheduled Job — run every hour

```
| trigger.schedule --cron "0 * * * *"
| http.request --url https://api.example.com/feed --method GET
| script -- "return input.body.items.slice(0,10)"
| sekejap.query --table feed_cache --op upsert
```

### Sekejap CRUD — read from embedded database

```
| trigger.webhook --path /api/notes --method GET
| sekejap.query --table notes --op scan
| script -- "return { notes: input }"
```

---

## Node Quick Reference

| Node | Use |
|------|-----|
| `trigger.webhook --path /x --method GET` | HTTP endpoint |
| `trigger.schedule --cron "* * * * *"` | Cron job |
| `pg.query --credential slug -- "SQL"` | PostgreSQL query |
| `sekejap.query --table x --op query/upsert` | Sekejap embedded DB (graph, vector, full-text) |
| `script -- "return {...}"` | JS transform |
| `http.request --url x --method GET/POST` | Outbound HTTP |
| `web.render --template-path pages/x.tsx --route /x` | Render HTML page |
| `logic.if --expr "input.role === 'admin'"` | Conditional branch |
| `logic.switch --expr "input.type" --cases a,b --default c` | Multi-branch |

---

## Next Steps

- `help_nodes` — full node catalog with all flags and examples
- `help_rwe` — how to write TSX templates that receive pipeline data
- `help_examples` — full archetype recipes (blog, chat, game, scheduling, scraping, auth)
