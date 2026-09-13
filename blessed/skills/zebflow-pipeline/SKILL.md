---
name: zebflow-pipeline
description: Building or changing a Zebflow pipeline — a route, an API endpoint, a page's data, a form's POST, a scheduled job, a webhook. Use before writing any DSL; covers the pair (pipeline + template) as the unit of done, flags, payload shape, register/activate, and the checks that prove a route works.
license: MIT
metadata:
  version: "1"
---

# Ship a pipeline

A pipeline is a trigger, nodes, and an answer. A route that renders a page is
a **pair**: the pipeline and the template it names. Half a pair is not done.

Facts live in `help(topic="pipeline")`, `pipeline/dsl`, `pipeline/authoring`,
`pipeline/web` and, for any node, `help(topic="pipeline/nodes/<kind>")`.

## Before writing a line

1. `pipeline_list` — does a pipeline for this route already exist? Extend it,
   do not add a second one on the same path.
2. For each node you will use for the first time this session:
   `help(topic="pipeline/nodes/<kind>")`. Flags are declared per node and an
   undeclared flag is a parse error — `--params` exists on the query nodes,
   `--route` exists on nothing, `--table`/`--op` never existed.
3. Names you must not guess: `--template` from `file_list` (ends in `.tsx`),
   `--credential` from `credential_list` (an id, not a connection slug), a
   table from `connection_describe`.

## Write it

- **Pipe mode** for a chain; **graph mode** (`[id]` and `->`) the moment you
  branch, fan out or loop. Every node must be reachable from the one entry —
  an unwired node is a second entry that fires on every request.
- **Payload shape.** After `trigger.webhook`: `input.body` — the JSON body,
  or for a `<form method="post">` an object of its fields (`<input name="email">`
  → `input.body.email`); `null` on GET — plus `input.params`, `input.query`,
  `input.files.<field>` (FileRef), `input.auth` when the trigger verified a
  token. After a query node: `{ columns, rows, … }` — the rows are
  `input.rows`, objects keyed by column (`input.rows[0].title`), never `input`.
  After `crypto` hash/encode ops: the same payload plus `result`
  (`input.result`, `input.body` still there). Reach an earlier node's output
  with `$nodes.<id>` in `{{ }}`.
- **SQL in the body, values in `--params`:**
  `sekejap.query --params "{{ [input.body.email] }}" -- "SELECT * FROM users WHERE email = $1"`.
  Never interpolate a value into SQL text.
- **A script returns the next payload and nothing else.** It cannot set a
  status or a header, `return null` does not stop the pipeline, and
  `setTimeout` / `fetch` are blocked in it. Branch with `logic.if --expr`,
  answer with `web.response`, call out with `http.request`.
- **`web.response`** decides the response: nothing → JSON of the payload;
  `--template pages/x.tsx` → the page; `--location` → redirect;
  `--status`, `--set-cookie "…"`, `--header K=V`. A 404 is a `logic.if` with
  two `web.response` nodes on its pins.
- **Forms are two pipelines.** `GET` renders the page; `POST` validates
  `input.body`, writes, then `--location` back (browser) or answers JSON
  (fetch). Both carry the same `--auth-*` flags.

```
register api/posts/create --title "Create post"
[a] trigger.webhook --path /api/posts --method POST --auth-type jwt --auth-credential jwt_main --auth-required-role editor
[b] logic.if --expr "typeof input.body?.title === 'string' && input.body.title.length > 0"
[c] sekejap.query --params "{{ [input.body.title, input.body.slug] }}" --read-only false -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
[d] web.response --location /admin/posts
[e] web.response --status 400 --body "{{ { error: 'title is required' } }}"
[a] -> [b]
[b]:true -> [c]
[c] -> [d]
[b]:false -> [e]
```

## Register, activate, prove

```
pipeline_register  file_rel_path="api/posts/create"  body="…"     → draft
pipeline_activate  file_rel_path="api/posts/create"              → active
```

`file_rel_path` is relative to the source root (the repository root unless
`zebflow.yaml` says otherwise) and is the pipeline's place in the project's
layout (`docs/structure.md`): `api/…`, `pages/…`, `jobs/…` in a flat project,
`modules/<domain>/api/…` in a domain one (`zebflow-engineering`) — never a
`pipelines/` prefix. The URL is `--path`, independent of the file's folder. To change one node later: `pipeline_describe` (node ids `n0, n1, …`)
→ `pipeline_patch node_id=` → `pipeline_activate` again; until then the
status is `stale` and traffic runs the old snapshot.

**The gate — none of these may be skipped:**

1. `pipeline_list status=all` shows the pipeline as `active`.
2. Fetch the route and read what came back:
   - a page: the body has no `RWE component error` and shows the data;
   - JSON: the shape you documented, with the status you meant;
   - a redirect: `303`/`302` to the right place, with the cookie if you set one;
   - the failure paths too: the 400 branch, the unauthenticated request.
3. `pipeline_get_invocations file_rel_path=…` shows the run with no error
   and the node trace you expected. A schedule that renders HTML, or a webhook
   that ran a node twice, shows up here and nowhere else.
4. If the route renders a page, continue with `zebflow-verify` — the server
   HTML being right says nothing about hydration.

To try a body without saving: `pipeline_run body="| trigger.function | …" input={…}`.

## When it fails

- `unknown flag --x` — the node does not declare it; `help(topic="pipeline/nodes/<kind>")`.
- `looks like an unquoted expression` — quote the whole value.
- the template is "not found" — the path in `--template` is not an exact
  `file_list` path, or lacks `.tsx`.
- `input.title is undefined` in a script after a webhook — it is `input.body.title`.
- rows missing after a query — you read `input`, the rows are `input.rows`.
- the route answers the old behaviour — the pipeline is `stale`; activate.
