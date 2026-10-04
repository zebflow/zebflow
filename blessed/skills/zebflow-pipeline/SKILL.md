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
   is any upstream answer (`null` when that node was on a branch not taken).
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
- **A node runs once,** when every edge into it has delivered or been
  skipped: branches meeting is a join, a branch not taken is skipped with
  everything only it feeds, and a loop's body runs per item until the
  `logic.reduce` / `logic.collect` that closes it. No cycles but a
  `logic.retry` going round. `web.response.send` answers at once; the rest
  of the run keeps going (`help("pipeline/dsl")`, Control flow).
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

## Check, register, run, fetch

Move bit by bit: each step answers before the next one is taken.

```
pipeline_check     body="…"                                      → every problem, or "No problems"
pipeline_register  file_rel_path="api/posts/create"  body="…"     → draft
pipeline_activate  file_rel_path="api/posts/create"              → active
route_fetch        path="/api/posts"  method=POST  form={…}      → what the route answers
```

`pipeline_check` saves nothing. It lists everything `pipeline_register` would
refuse — an unknown kind or flag (with the one it likely meant), a missing
required flag, a word outside a closed choice (`--format jpeg` → jpg, png,
webp), a duration or size without its unit, a cycle — and, as warnings, every
`input.<key>` or `$nodes.<id>.<key>` no upstream node answers, with the key it
meant (`input.result` after a script → `input.script`). Registration refuses
the first kind with all of them listed; fix every one, check again, then
register. A value still written as `{{ }}` is judged when it resolves, at run.

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
2. `route_fetch path=…` (POST with `form=`, an upload with `files=`,
   protected routes with `cookie=`) and read what came back:
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

## Publishing an MCP server

An app can publish functions to outside agents (ChatGPT, Claude, any MCP
client) as an MCP server — the app's, not this session's.

```
| trigger.mcp --route /shop --name search --description "Search the catalog by words. Use before quoting a product." --parameter q:string! "The words to look for." --auth api_key --credential shop-mcp-key
| sekejap.query.run --param "1={{ input.mcp.arguments.q }}" -- "SELECT sku, name FROM products WHERE name LIKE '%' || $1 || '%'"
| web.response.send --body "{{ input.query.rows }}"
```

- **A route is one server.** Every active `trigger.mcp` with the same
  `--route` is one of its tools; `--name` is unique on the route (a second is
  refused at activation). Another route is another server.
- **`--auth` is required and the same on every tool of a route**: `none`
  publishes openly and must be written; `api_key` or `jwt` take
  `--credential` (an id from `credential_list`).
- The arguments are `input.mcp.arguments.<name>` (`$trigger.arguments.<name>`
  later); the tool result is what `web.response.send` answers, and a status of
  400 or more is a tool error with that body.
- It will answer at `/_mcp/shop` on the project's hosts
  (`/mcp/{owner}/{project}/shop` on the platform), once the owner switches the
  `mcp` surface on in Settings → Addressing; it is off by default. Ask; never
  assume. **In 0.11 the declaration is checked (`pipeline_check`, activation)
  but the route is not served yet** — serving lands in 0.11.1.
- **This session never lists or calls a published tool.** Once served, a
  published route is proven as a webhook is: by calling the route.

## When it fails

- `unknown flag --x` — the node does not declare it; the refusal names the
  flags it takes (and the one you likely meant); `help(topic="pipeline/nodes/<kind>")`
  is the whole node.
- `Unknown node kind` — the refusal lists the kinds you likely meant;
  `help(topic="pipeline/nodes")` is the index.
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
