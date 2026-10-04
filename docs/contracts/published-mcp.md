# Published MCP

Status: **decided for 0.11** — 2026-10-04, by the owner. Code lands in 0.11
(ledger: `zebflow › … › zf-0-11-grammar`).

A project has two MCPs, and they are never mixed:

| | Project MCP (dev) | Published MCP |
| --- | --- | --- |
| What it is | the developer surface an agent uses to **build** the project | functions the project's **app** publishes to outside agents (ChatGPT, Claude, any MCP client) |
| Like | the Studio API | a webhook |
| Tools | `start_here`, `help`, `pipeline_register`, `file_write`, … | exactly the `trigger.mcp` pipelines on that route, nothing else |
| Address | `/api/projects/{o}/{p}/mcp` on the platform only — never on a project host | the `mcp` surface ([Addressing](./addressing.md) §2): `/_mcp/ROUTE` on the project's hosts, `/mcp/{o}/{p}/ROUTE` on the platform; **off** by default |
| Who may call | a developer's MCP session | whoever the route's `--auth` admits |

## Declaring one

```
| trigger.mcp --route /shop --name search --description "Search the catalog by text"
              --parameter "q:string! the words to look for" --auth api_key --credential shop-mcp-key
| postgres.query.run --credential shop --param "1={{ input.mcp.arguments.q }}" -- "SELECT … WHERE name ILIKE '%' || $1 || '%'"
| web.response.send --body "{{ input.query.rows }}"
```

- **A route is one MCP server.** Every active `trigger.mcp` with the same
  `--route` is one tool of that server; another `--route` is another server
  with its own tools. In one project, `--route /catalog` and
  `--route /support` publish two servers that share nothing.
- `--name` is the tool name, unique on its route (a second is refused at
  activation, naming both pipelines); `--description` is what the agent reads;
  `--parameter name:type[!] "doc"` (repeat) declares its arguments, as on
  `trigger.function`.
- **`--auth` is required** — publishing is never implicit. `none` must be
  written to publish openly; `jwt`, `api_key` (`--credential`, `--role`) as on
  `trigger.webhook`. Every `trigger.mcp` on one route declares the same auth,
  or activation refuses the route. The sign-in the ChatGPT and claude.ai
  connectors use (OAuth: code + PKCE, client registration, consent) is a later
  `--auth` word on the same routes.
- It answers `mcp: { route, tool_name, arguments }` (§6 of
  [Node Conventions](./node-conventions.md)); `$trigger` is that envelope.
- **The tool result** is what `web.response.send` answers (its `--body`), or the
  run's value without one; a status of 400 or more is a tool error with that
  body.

## Rules

- **A published MCP never uses Zebflow's own auth** — no platform account,
  Studio session, dev MCP session or project member role is ever accepted on,
  required by or consulted for a published route. It authenticates only with
  the **app's** mechanism, as the app's webhooks do: `api_key` checks a key the
  app keeps in its credentials; `jwt` checks a token the app signed
  (`auth.token.create`) and `--role` reads that token's roles; the later
  `oauth` signs people in through the app's own login and users, and its
  tokens are the app's tokens.
- A published route serves the MCP protocol for its own tools only: no project
  tool, resource, prompt or skill is ever listed or callable on it, and it
  reaches the project only through what its pipelines do.
- The project MCP never lists or calls published tools. A developer tests a
  published route as they test a webhook — by calling the route.
- Published routes live on their own surface, `mcp`, never under the webhook
  paths: knowing a project's pages and webhooks says nothing about its MCP.
  The surface is **off** by default, so publishing takes two explicit acts —
  switching the surface on and declaring `--auth` on each route. A route's
  `--errors` and the surface's hosts work as for `trigger.webhook`.
- Nothing is published by default: a project with no active `trigger.mcp`
  publishes no MCP at all.
