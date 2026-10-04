# Published MCP

Status: **decided for 0.11** — 2026-10-04, by the owner. Defined in 0.11.0, served from 0.11.1
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
  or activation refuses the route. `oauth` (below) signs people in through the
  app's own login, for the ChatGPT and claude.ai connectors.
- It answers `mcp: { route, tool_name, arguments }` (§6 of
  [Node Conventions](./node-conventions.md)); `$trigger` is that envelope.
- **The tool result** is what `web.response.send` answers (its `--body`); a
  status of 400 or more is a tool error with that body. A run that reaches no
  `web.response.send` answers an empty result, `{ content: [], isError: false }`
  — the run's value is never sent on its own (the webhook's `204` rule,
  [Node Conventions](./node-conventions.md) §4), and `pipeline_check` warns on
  every path from `trigger.mcp` that ends without one.

## `--auth oauth`

```
| trigger.mcp --route /library --name search --auth oauth --credential library-jwt --login /auth/login --role reader
# the app's login pipeline, after it has checked the person:
| auth.token.create --credential library-jwt --claim "sub=…" --claim "roles=…"
| auth.oauth.approve --ticket "{{ $trigger.body.oauth }}" --token "{{ input.token.access_token }}" --client "{{ $trigger.body.client }}" --redirect-host "{{ $trigger.body.redirect_host }}"
```

- The route is an OAuth 2.1 protected resource and its own authorization
  server (MCP authorization 2026-07-28, compatible with 2025-11-25);
  `resource` = `issuer` = **R**, the connect URL: `https://{host}{mount}{route}`
  on a named host, `http://{host}:{port}…` on the dev host,
  `{ZEBFLOW_PLATFORM_BASE_URL}/mcp/{o}/{p}{route}` on the platform form —
  which has no OAuth when that is unset. Never read off a forwarded header.
- `--credential` is a `jwt_signing_key` (refused at activation otherwise): it
  verifies the app's token at approve and signs the access tokens.
- `--login` is a path of the app's `pages`, never a URL, never `/_…`. Authorize
  sends the person there with `?oauth=<ticket>&client=<name>&redirect_host=<host>`;
  the page shows the last two and posts all three back; `auth.oauth.approve`
  refuses unless they are the ticket's (§6 of [Node Conventions](./node-conventions.md)).

| Document / endpoint | Where |
| --- | --- |
| protected resource metadata (RFC 9728) | `/.well-known/oauth-protected-resource` + the path of R |
| authorization server metadata (RFC 8414) | `/.well-known/oauth-authorization-server` + the path of R |
| authorize · token · register (RFC 7591) | `R/_oauth/authorize` · `R/_oauth/token` · `R/_oauth/register` |

- A route segment starting with `_` is refused at activation.
- No token: 401 with `WWW-Authenticate: Bearer resource_metadata="…", scope="mcp"`
  (`error="invalid_token"` when one was sent); a missing role is 403.
- Clients: a client ID metadata document (an `https` `client_id`, fetched
  under the outbound policy) or RFC 7591 registration; public clients only,
  PKCE S256 only, `resource` (RFC 8707) checked, `iss` on every authorization
  response (RFC 9207), redirect URIs matched exactly (a loopback port aside).
- The access token is a JWT (`typ: at+jwt`) of the approved app token's claims
  plus `iss`, `aud` = R, `exp` (1 h), `jti`, `client_id`, `scope`; it opens this
  route only — never another route, never a `jwt` webhook.

## Rules

- **A published MCP never uses Zebflow's own auth** — no platform account,
  Studio session, dev MCP session or project member role is ever accepted on,
  required by or consulted for a published route. It authenticates only with
  the **app's** mechanism, as the app's webhooks do: `api_key` checks a key the
  app keeps in its credentials; `jwt` checks a token the app signed
  (`auth.token.create`) and `--role` reads that token's roles; `oauth` signs
  people in through the app's own login page and issues the app's JWTs. No
  Zebflow cookie is read or set anywhere in its flow.
- OAuth state lives in the project's durable store under `zf.oauth/…` (no
  `kv.*` node takes a `zf.` key), secrets only as hashes: tickets 10 min and
  codes 2 min, each used once; refresh tokens rotate, and one presented twice
  revokes its family; registrations and authorizations are capped per route
  (`MAX_REGISTRATIONS_PER_DAY`, `MAX_TICKETS_PER_MINUTE` in
  `services/published_oauth`).
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
