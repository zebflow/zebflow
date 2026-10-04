---
name: zebflow-pipeline
description: Building or changing a Zebflow pipeline — a route, an API endpoint, a page's data, a form's POST, a scheduled job, a webhook. Use before writing any DSL; covers the node grammar (family.noun.verb, one answer key, --from), the pair (pipeline + template) as the unit of done, register/activate, and the checks that prove a route works.
license: MIT
metadata:
  version: "1"
---

# Ship a pipeline

A pipeline is a trigger, nodes, and an answer. A route that renders a page is
a **pair**: the pipeline and the template it names. Half a pair is not done.

The grammar is `help(topic="pipeline/dsl")`; what one node takes and answers
is `help(topic="pipeline/nodes/<kind>")`, generated from its definition. This
skill is the order of work.

## The grammar in five lines

1. A node is written as its kind: `family.noun.verb` (`fs.file.put`,
   `postgres.query.run`, `telegram.message.send`); entry nodes are
   `trigger.<source>`, inputs `input.<type>`, control `logic.<verb>`.
2. Every node adds **one key — its noun** — and keeps the rest of the payload:
   the trigger adds `webhook`, a query `query`, `fs.file.put` `file`, a script
   `script`. `$trigger` is the request for the whole run; `$nodes.<id>.<key>`
   is any earlier answer.
3. `--from` names the subject; other inputs are typed roles (`--text`,
   `--file`, `--body`, `--value`, `--argument`); maps are repeated
   `key=value` (`--param "1=…"`, `--header "K=V"`); switches are bare
   (`--write`); units are in values (`--timeout 30s`, `--max-size 10MB`);
   choices are closed words.
4. Statements go in the body after `--`; values bind through `--param`; a
   node that changes data needs `--write`.
5. Headers are sent exactly as written — a cookie writes its own attributes.

## Before writing a line

1. `pipeline_list` — does a pipeline for this route already exist? Extend it;
   do not add a second one on the same route.
2. Find each node: `help(topic="pipeline/nodes")` (one line per kind) →
   `help(topic="pipeline/nodes/<kind>")` for every node you use for the first
   time this session → `help_search query="…"` when you only know the task.
   A node accepts only the flags it declares; an undeclared flag is a parse
   error.
3. Names you must not guess: `--template` from `file_list` (ends in `.tsx`),
   `--credential` from `credential_list` (an id, never a connection slug), a
   table from `connection_describe`.

## Write it

- **Pipe mode** for a chain; **graph mode** (`[id]` and `->`) the moment you
  branch, fan out or loop. Every node must be reachable from the one entry —
  an unwired node is a second entry that fires on every request.
- **Read the request where it is.** Right after `trigger.webhook` it is
  `input.webhook` — the body (`input.webhook.body.email`, `null` on GET),
  `.params`, `.query`, `.files.<field>` (FileRefs), `.auth` when the trigger
  verified a token. Anywhere later in the run it is `$trigger.body.email`,
  `$trigger.params.slug`, `$trigger.auth.sub`: the request never moves.
- **Read an answer under its node's noun.** After a query the rows are
  `input.query.rows` (`input.query.rows[0].title`); after `fs.file.put` the
  FileRef is `input.file`; a script's return is `input.script`. The shape
  inside each key is on the node's page.
- **SQL in the body, values in `--param`:**
  `sekejap.query.run --param "1={{ $trigger.body.email }}" -- "SELECT * FROM users WHERE email = $1"`.
  Never build SQL text from input.
- **A script shapes data; it does not answer.** It cannot set a status or a
  header, `return null` does not stop the run, and `fetch` / `setTimeout` are
  blocked in it. Branch with `logic.if --when`, answer with
  `web.response.send`, call out with `http.response.fetch`.
- **`web.response.send`** decides the response: no flag → the payload as JSON;
  `--template pages/x.tsx` → the page, the payload as its `input`;
  `--body VALUE` → text or JSON; `--status 303 --header "Location=/path"` → a
  redirect (always root-relative — the project's host makes it right);
  `--header "Set-Cookie=…; Path=/; HttpOnly"` → a cookie, sent as written. A
  404 is a `logic.if` with two `web.response.send` nodes on its pins.
- **A site-wide 404 is `trigger.error --status 404 | web.response.send --status 404 --template pages/not-found.tsx`.**
  A webhook on `/*` does not catch unknown routes.
- **Forms are two pipelines.** `GET` renders the page; `POST` validates
  `input.webhook.body`, writes, then redirects with
  `--status 303 --header "Location=…"` (a browser) or answers JSON (a
  `fetch`). Both carry the same auth flags (`--auth`, `--credential`, `--role`).

```
[a] trigger.webhook --route /api/posts --method POST --auth jwt --credential jwt_main --role editor
[b] logic.if --when "typeof input.webhook.body?.title === 'string' && input.webhook.body.title.length > 0"
[c] sekejap.query.run --write --param "1={{ $trigger.body.title }}" --param "2={{ $trigger.body.slug }}" -- "INSERT INTO posts (title, slug) VALUES ($1, $2)"
[d] web.response.send --status 303 --header "Location=/admin/posts"
[e] web.response.send --status 400 --body "{{ { error: 'title is required' } }}"
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
`pipelines/` prefix. The URL is `--route`, independent of the file's folder.
To change one node later: `pipeline_describe` (node ids `n0, n1, …`) →
`pipeline_patch node_id=` → `pipeline_activate` again; until then the status
is `stale` and traffic runs the old snapshot.

**The gate — none of these may be skipped:**

1. `pipeline_list status=all` shows the pipeline as `active`.
2. `route_fetch path=…` (POST with `form=`, protected routes with `cookie=`)
   and read what came back:
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

- `unknown flag --x` — the node does not declare it; read
  `help(topic="pipeline/nodes/<kind>")` and use the word it lists.
- a word refused for a choice (`--method`, `--fit`, `--on-conflict` …) — use one
  the node's signature lists; nothing is mapped to a default.
- a duration or size refused — write the unit (`30s`, `10MB`).
- `looks like an unquoted expression` — quote the whole value.
- the template is "not found" — the path in `--template` is not an exact
  `file_list` path, or lacks `.tsx`.
- `undefined` in a script right after a webhook — the field is
  `input.webhook.body.title` (or `$trigger.body.title`).
- rows missing after a query — they are under the query's key,
  `input.query.rows`.
- the route answers the old behaviour — the pipeline is `stale`; activate.
