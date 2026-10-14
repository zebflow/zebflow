# Addressing

Status: **Implemented (v1)** — 2026-09-13; §2c sections implemented —
2026-10-14. Where a project answers on the network, and how a URL
written inside it stays right wherever it answers.

## 0. The rule

**A project never contains its own address.** Not in a page, a pipeline,
`zebflow.yaml`, a bundle, or a hub package. Where it answers is instance
configuration, set by the operator, changed without touching the project.

## 1. Where a project answers

```
hosts
  <project>.<owner>.localhost:<port>      dev · automatic · every project, always
  <root>.<project>.<owner>.localhost      dev · automatic · one per section root (§2c)
  <any domain the owner adds>             prod · Settings → Addressing · DNS is the owner's
neutral form
  /wh/{owner}/{project}{path}             always on · the Studio and tools · never the site
```

A request whose `Host` is one of a project's hosts is that project's, and
its path is an app path (`/book`). Browsers resolve `*.localhost` to loopback
on their own — dev needs no DNS and no hosts file. A proxy in front passes
`Host` through and does nothing else. **No public response names the owner
or project** — no header, and no page's hydration state (§1a). Verify proves a
host with a one-time probe: it mints a token for the project and requests
`/_verify/{token}` through the host, which answers `204` only when the host
resolved to that project (`404` with `x-zebflow-verify: other` otherwise).
`/_verify` alone answers `204` on any project host — the check a generated
proxy config ends with (`web/host_verify.rs`).

## 1a. What a page publishes

A rendered page's hydration state (`<script id="__rwe_payload">`) never
carries the request: no request headers (the visitor's address, a proxy's
internal `x-forwarded-for` / `x-real-ip` chain, `host`, `cookie`, the user
agent), no cookies, and no trigger envelope's `auth`; the root `auth` is only
the claims the token marks `_zf_public`. Server-side rendering still reads
them. A page's route is the path the visitor asked for — on a project host
`/book`, never the `/wh/{o}/{p}/book` it is served as. One function decides
this for every render and for `web.site.generate`
(`rwe/core/page_state.rs`). A page that wants a request value in the browser
copies it into its own key upstream.

## 2. Surfaces and routes

A **route** is `(host, path prefix) → surface`. A project is born with the
default routes below on every host it has; the operator may add routes that
mount a surface on a host and path of their choosing (`zebms.mydomain.com/service/`).
A host that carries any custom route serves only its routes, except a section
host (§2c): a path none of them match is 404, never a platform route (the
project's own platform forms pass, as on any of its hosts).

| Surface | Default | Default path on a project host | Platform form (valid on every host) |
|---|---|---|---|
| `pages` — active `trigger.webhook` routes | on | `/` (**root**; the only surface that can be) | `/wh/{o}/{p}/…` |
| `ws` — `trigger.room` rooms | on | `/_ws/rooms/{room}` | `/ws/{o}/{p}/rooms/{room}` |
| `files` — objects [`ZebFsAcl`](./kinds/zebfs-acl/README.md) exposes, through the ZebFS gateway | **off** | `/_files/…` when switched on; always on the project's file hosts (§2b) | — (no platform form) |
| `static` — project assets, `_rwe/lib`, `_rwe/scripts` | on | `/_static/…` | `/static/{o}/{p}/…` |
| `ms` — published map layers | **off** | `/_ms/…` | `/ms/{o}/{p}/…` |
| `mcp` — the app's **published** MCP servers, one per `trigger.mcp --route` ([Published MCP](./published-mcp.md)) | **off** | `/_mcp/…` | `/mcp/{o}/{p}/…` |

The project's **dev** MCP (the tools an agent uses to build the project) is
not a surface: it answers only at `/api/projects/{o}/{p}/mcp` on the platform
address and never on a project host.

`files` answers only what `ZebFsAcl` exposes, through the ZebFS gateway
(inert, no cookie): always on the project's file host, and on a project host
under `/_files/…` only when switched on (off by default). The `fs` surface and
the platform forms `/files/…` and `/fs/…` are deleted (2026-10-03). Exposure
is only what `ZebFsAcl` says; `public_read` answers on the cookie-less file
host, `public_execute` on the hosts in its `serve`, each of which answers only
that folder (as a host with a custom route does).

Rules: `/_` is reserved — a webhook path starting with it is refused at
register (`PIPELINE_ROUTE_RESERVED`). `pages` mounts at `/` or nowhere
(`ADDRESSING_PAGES_ROOT`), except on a section host (§2c); every other
surface mounts anywhere. A disabled
surface answers 404 on the project's hosts and on its platform form alike.
A host belongs to one project on an instance (`ADDRESSING_HOST_TAKEN`).

There is no dev mode and no production mode: Zebflow is one runtime, and no
behaviour keys on which host asked — the dev host and a named host obey the
same switches. What a visitor gets is what the switches say, and the switches
can be read back.

## 2a. Switches

Beside `hosts`, `routes` and `disabled`, the project's addressing record
(`addressing.json`, store tier, so it never travels with the code) carries:

| Switch | Default | Meaning |
|---|---|---|
| `api_on_hosts` | `false` | the platform API `/api/projects/{o}/{p}/…` answers on the project's hosts (dev host included). The platform address always serves it — that is where the Studio lives. Authentication applies either way. The dev MCP (`…/mcp`) is excluded: it never answers on a project host |
| `mcp` (in `disabled`) | off | the published MCP servers: `/_mcp/ROUTE` on the project's hosts and `/mcp/{o}/{p}/ROUTE` on the platform — both off until switched on |
| `errors` | `hidden` | what a **5xx** shows on the project's hosts: `hidden` — the project's 500 page (or the platform's neutral one) with the first eight characters of the run id, and `{ "error": { "code": "internal", "request_id": … } }` for a JSON request; `shown` — the same page plus the error code, message, node id and a link to the run. A webhook overrides its own routes with `--errors show` or `--errors hide` (`kinds/pipeline`). Status codes never change with this switch, and an authored 4xx (`web.response.send --status 400 --body …`) always shows its message |

The full detail of every failure is in the invocation record and its error
group (`kinds/invocation-record`) whatever `errors` says; `hidden` hides, it
never loses. Decided 2026-09-17; the dev-host exception that briefly served
the API on `*.localhost` regardless of the switch is withdrawn by this rule.

## 2b. File hosts

```
<project>.<owner>.fs.localhost      dev · automatic · every project
<any host> + route  / → files       named · Settings → Addressing · DNS is the owner's
```

A host carrying a `files` route is a **file host**: the ZebFS gateway at its
root, inert and cookie-less ([`ZebFsAcl`](./kinds/zebfs-acl/README.md)), and
nothing else — no page, no platform form, GET and HEAD only. It answers
whatever the `files` switch says; the switch governs only the `/_files/` mount
on site hosts. The route is the host's only route, at `/`, on a named host
(`ADDRESSING_FILE_HOST`). A file host runs nothing: a `public_execute` rule
may not name it in `serve`, and saving a route that turns a served host into
a file host drops it from every `serve`.

Production uses a host of its own (`files.example.com`) under the project
domain. The project's site keeps host-only cookies, so a file host shares no
session with it; a separate registrable domain is the step after, when
anonymous uploads arrive at scale.

## 2c. Sections — one project, several sites

**Opt-in.** A project whose routes carry no `root` behaves exactly as
§1–§2b say, on today's code path at today's cost: every branch below is taken
only after "this project has a root" is true. A project that has one is a
**sectioned project**.

**Yardstick.** A sectioned project is never weaker than the same sites built
as separate projects on the same hosts. What separate projects would not have
— one site reaching another's paths — is closed here. What they would have too
— sibling-host cookies, cross-site form posts — is every site's own concern
and not changed by sections. What sections share on purpose: data,
credentials, pipelines (a parameter route `/{s}/map` is the section's own path
on its host), the error pages and `errors`, `api_on_hosts`, the ZebFS rules
and file hosts, `static`, `ms` and rooms. A section that must refuse another's
tokens signs with its own credential.

```json
"hosts":  ["example.org", "a.example.net"],
"routes": [
  { "host": "a.example.net", "path": "/",     "surface": "pages", "root": "/site-a" },
  { "host": "a.example.net", "path": "/api/", "surface": "pages" }
]
```

A host whose `/` pages route carries a root is a **section host**; that root
is its section. A root is stored as `/` plus one DNS label (`[a-z0-9]`, inner
`-`, ≤ 63); the dev label is the root without its slash. A root may stand only
on a section host's `/` pages route and on its `mcp` route. Mapping on a section host:

- `pages`: app path = `root + visitor path`, nothing stripped. A further pages
  route carries no root and keeps its path, never under `/_`: it is shared
  (`/api/` answers `/api/…`).
- `mcp`: published route = `root + (visitor path − mount)`. Without an `mcp`
  route the host mounts `/_mcp/` with its root; an explicit `mcp` route carries
  the host's root or none (shared). Within `mcp`, a protocol suffix
  (`/_oauth/…`) is split off before any route rule applies.
- Every other surface it has no route for keeps its default mount (§2),
  project-wide, under its switch. `/assets/` (the platform's own icons and
  fonts) passes as on every host.

| Request | Answers |
|---|---|
| `a.example.net/` · `a.example.net/map` | webhook `/site-a` · `/site-a/map` |
| `a.example.net/api/data` | webhook `/api/data` (shared) |
| `a.example.net/_mcp/tools` | `trigger.mcp --route /site-a/tools` |
| `example.org/map` | webhook `/map` |
| `example.org/site-a/map` · `//site-a/map` · `/%73ite-a/map` · `/wh/{o}/{p}/site-a/map` | 404 (claimed) |
| `example.org/_mcp/site-a/tools` · `/mcp/{o}/{p}/site-a/tools` | 404 (claimed) |
| `a.example.net/wh/{o}/{p}/…` · `/mcp/{o}/{p}/…` · `/%77h/{o}/{p}/…` | 404 |
| `a.example.net/%2F…` · `/x/%2e%2e/y` | 400 |
| `site-a.northside.superadmin.localhost:10610/map` | webhook `/site-a/map` (dev) |
| platform address `/wh/{o}/{p}/site-a/map` | webhook `/site-a/map`, as today |

Rules for a sectioned project, on every host it has:

- **One reading of the path.** One function, shared by the gate and the
  matchers, reads the path: decoded once, empty segments dropped. An encoded
  `/`, `\` or NUL, or a `.` or `..` segment, answers 400. Routing, the probe,
  the platform forms and the well-known documents are all matched on that
  form, and a pass-through is forwarded rebuilt from it by the Visitor form
  serializer, segments escaped, so nothing is decoded twice.
- **Claim.** An app path or a published MCP route whose first segment is a
  root answers only for an arrival on a route carrying that root, or on the
  **platform address**: the configured base URL's host, and always
  `localhost`, `127.0.0.1` and `[::1]` on any port (a tunnel names its own) — and
  never a host in the index, where the project-host reading wins (saving such
  a host on a sectioned project is refused as help). Any other host, a stray
  name or a request with no host included, is a rootless arrival, and a
  `/wh/` or `/mcp/` pass-through on a project host is an arrival on that host.
  The claim is decided in the gate, on the controller, before any forwarding.
  A sectioned request reaches a worker on a section-only internal route, the
  only place an arrival is read, and the worker routes it by that arrival,
  never by its own index; rootless forwarding and its headers
  (`x-zebflow-visitor-path`, `x-zebflow-mcp-base`) stay as today and gain
  none. Placing a project on another office is not built
  (`CLUSTER_REMOTE_PLACEMENT_UNBUILT`); until it is, a sectioned project's
  request that would be forwarded answers 503. A request sent straight to a worker's own address is that office's
  platform address, as for any project; one carrying a project host of a
  project the worker's index marks sectioned answers 404. A claimed request is forwarded with
  `claimed: true` in its arrival and answered by the ingress's not-found path,
  the project's own 404 with `original_path` the visitor path, as an unknown
  path is. A decoded
  segment right after a root that starts with `_` matches nothing, parameters
  included.
- **No escape.** The project's own `static`, `ws` and `ms` platform forms pass
  first, under their switches, on every host but a file or `serve` host; a
  section host passes no `/wh/` or `/mcp/` form; `/api/projects/…` follows
  `api_on_hosts`. Anything else no route of the host matches is the project's
  404, never a platform route.
- **Visitor form.** The platform writes some URLs from an app path: the
  `--auth` redirect target (`auth_redirect`, `auth_forbidden_redirect`) and
  its `next`, `--login`, a published MCP
  server's connect URL, issuer and well-known documents. They are written whole
  (`/site-a/login`) and emitted for the host that was reached, through the
  surface they point at: a pages target under the host's root loses the root,
  one under a shared route stays as it is, an MCP target gets its mount back
  (`/site-a/tools` under `/rpc/` → `/rpc/tools`). On the platform address they
  get the platform form. Anything else, or a result that does not route back
  to the same surface and app path on that host, is not emitted: the run fails
  with the reason; an `--auth` refusal answers its status with no `Location`;
  a document with no run answers 400; each records the misconfiguration.
  `trigger.error`'s `original_path` is the visitor path as typed, never
  checked. One serializer writes
  the path: one leading `/`, segments escaped, query kept, never `//`, `/\`
  or a dot segment. The arrival (scheme by the issuer's rule, host, port,
  surface, mount, root, visitor path, and that host's routes as the controller
  read them — the only routing emission consults) reaches a worker as one JSON
  value on the section-only internal route, proven as the controller's like
  today's forwarding; nothing a client sends is read as an arrival. Without it
  nothing is emitted. `pathname` is the
  visitor path; `$trigger.path` is the app path.
- **Dev host.** `<label>.<project>.<owner>.localhost` serves exactly the
  routes of the section hosts with that root, the `host` field aside; hosts
  sharing a root (aliases) must carry identical routes. It is claimed in the
  host index, under the index's own lock, at the moment the routing is
  published; if another project already holds that name the save fails
  (`ADDRESSING_HOST_TAKEN`), and a project's own named host of that name is
  dropped as its two-label dev host is; at boot a collision drops only the
  section dev host and is logged. It is a dev host for the `dev_host` flag and
  the issuer's rule (http, the listen port). The dev-host parser stays
  two-label. It is never a site host for
  `serve`. The `_verify` probe on a sectioned project's host proves the
  section too: the token is minted for one host and answers 204 only on a host
  of the same section.
  `route_fetch /site-a` goes to `site-a.<project>.<owner>.localhost/`.
- **Saving refuses** (`ADDRESSING_ROOT`): a malformed root or one on any other
  route; aliases with differing routes; a root that is the first segment of an
  unrooted pages route; a root in a project whose owner is `fs` (§2b's dev file
  host would be ambiguous); a section host that is a file host or a `serve`
  host. An addressing save also refuses while a registered webhook route or
  `--login` under a root has a visitor form the host sends elsewhere (another
  route, a platform form, the probe, the two OAuth well-known documents of an
  `mcp` path) or a segment after the root starting with `_`
  (`PIPELINE_ROUTE_SHADOWED`, `PIPELINE_ROUTE_RESERVED`). Registration adds no
  check: such a route registered later is unreachable on that host and its
  URLs are not emitted. Saving a root moves the project's routes already under
  it into the section; the answer lists them.
- **Saves add no lock.** The refusals above help the author; they are not the
  guarantee. The guarantee is the request rules, which fail closed whatever
  order saves land in: a host that is both a section host and a file or
  `serve` host answers 404 on the section's routes; a shadowed route is
  unreachable and its URLs are not emitted.
- **Rootless stays byte-identical.** `root` is omitted when absent, so a
  rootless addressing file round-trips unchanged; the one path-reading
  function, the 400s, the emitter and the arrival route run only behind "has
  a root"; registration is unchanged for every project.
- **One publishing index.** An addressing or ACL save is applied to the
  controller's index before it is answered, whatever office stores the file; a
  worker's index is never the publishing index.
- **Cost.** For a sectioned project only, the routing — each host's sorted
  routes, the root set, the section dev hosts, the `serve` hosts — is prepared
  wherever addressing enters the index (boot, first read, addressing or ACL
  save). A request costs a host lookup,
  prefix compares and one set lookup on the first segment; no disk, no body
  read or rewritten. A rootless project keeps today's path.

## 3. Two kinds of URL, opposite rules

| Kind | Written by | Form | Rule |
|---|---|---|---|
| **App URLs** — `href`, `action`, a `Location` header, `router.push`, client `fetch` | the author, the agent | root-relative, host-relative: `/book`, `/api/slots` | never carries owner, project or host; correct on every host in §1 |
| **Platform-emitted URLs** — `_rwe/lib/*`, `_rwe/scripts/*`, asset, file, room and tile URLs | the RWE and the platform, never a person | the platform form (`/static/{o}/{p}/…`) | unambiguous under every URL a page can be rendered at; on a project host the platform form of *that* project passes through untouched |

The first kind is what a website at a domain root writes anyway, which is
why an agent needs no rule to get it right. The second kind is the only
place owner and project appear, and code that knows them writes it.
(v1 emits the platform form everywhere; emitting a `static` route's host for
CDN caching is a later step and changes nothing above.)

## 4. What the tools do

- `route_fetch path=/book` sends `Host: <project>.<owner>.localhost` to
  loopback — the agent verifies what a browser gets, redirects included. A
  path under a root goes to that section's dev host, the root taken off
  (`/site-a/map` → `site-a.<project>.<owner>.localhost/map`, `/site-a` → `/`).
- The Studio's webhook URLs (`components/lib/addressing.ts`) use the dev host,
  or the section's dev host for a route under a root.
- A project bundle, a hub package, `git clone`, and `zeb transfer` carry no
  host; hosts are re-entered on the destination (`distribution.md`).
- `zebflow.yaml` has no address field. Hosts, routes and switches live in
  `users/{owner}/{project}/data/store/addressing.json` (store tier,
  `instance-directory.md`); `GET|PUT /settings/addressing` reads and writes
  it, `POST /settings/addressing/check {host}` resolves DNS and verifies.

## 5. Not covered

- **Sub-path mounting of pages** (`example.com/shop/` → a project's site). It
  cannot hold §0 without rewriting app URLs; refused. Other surfaces mount at
  sub-paths freely. §2c is the opposite direction: a root adds a prefix the
  visitor never sees, so no app URL changes.
- **Per-page or per-pipeline base paths.** There is no `{{ $base }}`.
- **TLS and DNS.** The proxy's and the owner's; the generated configs say how.

## 6. Operator surface — Settings → Addressing

The reference implementation of `ux.md` §4 (why · inputs · output · verify):

```
Your site            http://northside.superadmin.localhost:10610/   [Open] [Copy]
Production hosts     (why: a web server in front sends your domain here; the project does not change)
                     northside.example   DNS → 203.0.113.7 · answers from this project   [Verify] [Open] [Remove]
                     + Add host
Advanced ▾           routes: host/path → surface (dropdown of your hosts) · surfaces: one switch each, paths copyable
Web server config    [Nginx & OpenResty] [Apache httpd] [Caddy] [Traefik] [Cloudflare Tunnel] [HAProxy] [Docker Compose + Caddy]
                     generated from the hosts: Host header passed, WebSocket upgrade, upload limit, ends with its check
```

Templates: `pages/project-studio/settings/components/addressing/*`;
generators: `services/addressing.rs` `server_configs`.

## Evidence

- `services/addressing.rs` tests: dev-host parsing, default mounts, pages-root
  refusal, host validation, platform-path mapping, every generated config
  names every host, passes Host, ends with its check.
- Live: Sonnet's ladder site (written with `/book`, a `Location: /admin` redirect)
  works unchanged in a browser at `ladder-sonnet.superadmin.localhost:10610`
  — login redirect, cookie, admin page; assets load; the neutral form still
  answers; an unknown project host is 404.
- §2c: `services/addressing/sections/tests.rs` — every row of the table,
  default mounts, `_` after a root, the visitor form, the serializer, every
  `ADDRESSING_ROOT` refusal, shadowed and reserved routes, the platform
  address. `tests/platform/sections.rs` — through the router: each host's
  site, every claimed spelling and form, the project's 404 with the visitor's
  path, 400s, the stray host, Verify, the login redirect and `next` on a
  section host and on the platform address, a rooted MCP server's connect
  URL, save refusals, removing the root. Rootless projects: the whole
  existing suite, unchanged.
- To add: a router test over three URLs for one page; `/_` refusal test;
  settings round-trip; the model ladder round 2 in a browser at the dev host.
