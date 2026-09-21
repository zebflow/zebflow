# Addressing

Status: **Implemented (v1)** — 2026-09-13. Where a project answers on the
network, and how a URL written inside it stays right wherever it answers.

## 0. The rule

**A project never contains its own address.** Not in a page, a pipeline,
`zebflow.yaml`, a bundle, or a hub package. Where it answers is instance
configuration, set by the operator, changed without touching the project.

## 1. Where a project answers

```
hosts
  <project>.<owner>.localhost:<port>      dev · automatic · every project, always
  <any domain the owner adds>             prod · Settings → Addressing · DNS is the owner's
neutral form
  /wh/{owner}/{project}{path}             always on · the Studio and tools · never the site
```

A request whose `Host` is one of a project's hosts is that project's, and
its path is an app path (`/book`). Browsers resolve `*.localhost` to loopback
on their own — dev needs no DNS and no hosts file. A proxy in front passes
`Host` through and does nothing else; every response served for a project
host carries `x-zebflow-project: {owner}/{project}`, which is what Verify
reads back through the public host.

## 2. Surfaces and routes

A **route** is `(host, path prefix) → surface`. A project is born with the
default routes below on every host it has; the operator may add routes that
mount a surface on a host and path of their choosing (`zebms.mydomain.com/service/`).
A host that carries any custom route serves only its routes.

| Surface | Default | Default path on a project host | Platform form (valid on every host) |
|---|---|---|---|
| `pages` — active `trigger.webhook` routes | on | `/` (**root**; the only surface that can be) | `/wh/{o}/{p}/…` |
| `ws` — `trigger.ws` rooms | on | `/_ws/rooms/{room}` | `/ws/{o}/{p}/rooms/{room}` |
| `files` — public objects (the store's `public/` folder) | yes | `/_files/…` → `public/…` | `/files/{o}/{p}/public/…` |
| `static` — project assets, `_rwe/lib`, `_rwe/scripts` | on | `/_static/…` | `/static/{o}/{p}/…` |
| `ms` — published map layers | **off** | `/_ms/…` | `/ms/{o}/{p}/…` |
| `fs` — private objects (session) | **off** | `/_fs/…` | `/fs/{o}/{p}/…` |
| `mcp` | **off** | `/_mcp` | `/api/projects/{o}/{p}/mcp` — always served on the platform address |

Rules: `/_` is reserved — a webhook path starting with it is refused at
register (`PIPELINE_ROUTE_RESERVED`). `pages` mounts at `/` or nowhere
(`ADDRESSING_PAGES_ROOT`); every other surface mounts anywhere. A disabled
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
| `api_on_hosts` | `false` | the platform API `/api/projects/{o}/{p}/…` answers on the project's hosts (dev host included). The platform address always serves it — that is where the Studio lives. Authentication applies either way |
| `mcp` (in `disabled`) | off | `/_mcp` on the project's hosts; the platform form is always served on the platform address |
| `errors` | `hidden` | what a **5xx** shows on the project's hosts: `hidden` — the project's 500 page (or the platform's neutral one) with the first eight characters of the run id, and `{ "error": { "code": "internal", "request_id": … } }` for a JSON request; `shown` — the same page plus the error code, message, node id and a link to the run. A webhook overrides its own routes with `--errors show` or `--errors hide` (`kinds/pipeline`). Status codes never change with this switch, and an authored 4xx (`web.response --status 400 --message …`) always shows its message |

The full detail of every failure is in the invocation record and its error
group (`kinds/invocation-record`) whatever `errors` says; `hidden` hides, it
never loses. Decided 2026-09-17; the dev-host exception that briefly served
the API on `*.localhost` regardless of the switch is withdrawn by this rule.

## 3. Two kinds of URL, opposite rules

| Kind | Written by | Form | Rule |
|---|---|---|---|
| **App URLs** — `href`, `action`, `--location`, `router.push`, client `fetch` | the author, the agent | root-relative, host-relative: `/book`, `/api/slots` | never carries owner, project or host; correct on every host in §1 |
| **Platform-emitted URLs** — `_rwe/lib/*`, `_rwe/scripts/*`, asset, file, room and tile URLs | the RWE and the platform, never a person | the platform form (`/static/{o}/{p}/…`) | unambiguous under every URL a page can be rendered at; on a project host the platform form of *that* project passes through untouched |

The first kind is what a website at a domain root writes anyway, which is
why an agent needs no rule to get it right. The second kind is the only
place owner and project appear, and code that knows them writes it.
(v1 emits the platform form everywhere; emitting a `static` route's host for
CDN caching is a later step and changes nothing above.)

## 4. What the tools do

- `route_fetch path=/book` sends `Host: <project>.<owner>.localhost` to
  loopback — the agent verifies what a browser gets, redirects included.
- The Studio's webhook URLs (`components/lib/addressing.ts`) use the dev host.
- A project bundle, a hub package, `git clone`, and `zeb transfer` carry no
  host; hosts are re-entered on the destination (`distribution.md`).
- `zebflow.yaml` has no address field. Hosts, routes and switches live in
  `users/{owner}/{project}/data/store/addressing.json` (store tier,
  `instance-directory.md`); `GET|PUT /settings/addressing` reads and writes
  it, `POST /settings/addressing/check {host}` resolves DNS and verifies.

## 5. Not covered

- **Sub-path mounting of pages** (`example.com/shop/` → a project's site). It
  cannot hold §0 without rewriting app URLs; refused. Other surfaces mount at
  sub-paths freely.
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
- Live: Sonnet's ladder site (written with `/book`, `--location /admin`)
  works unchanged in a browser at `ladder-sonnet.superadmin.localhost:10610`
  — login redirect, cookie, admin page; assets load; the neutral form still
  answers; an unknown project host is 404.
- To add: a router test over three URLs for one page; `/_` refusal test;
  settings round-trip; the model ladder round 2 in a browser at the dev host.
