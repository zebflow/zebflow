# Several sites in one project (sections)

One project can serve several sites on their own domains while sharing one
database, one set of credentials and one set of pipelines. Each site is a
**section**: a host whose `/` pages route carries a `root`.
`docs/contracts/addressing.md` §2c is the full rule set.

## Set it up

Settings → Addressing → Advanced: add the host, then a `pages` route at `/`
with a root.

```json
"hosts":  ["example.org", "a.example.net"],
"routes": [
  { "host": "a.example.net", "path": "/",     "surface": "pages", "root": "/site-a" },
  { "host": "a.example.net", "path": "/api/", "surface": "pages" }
]
```

## Write the section's routes whole

A webhook in the section is registered with its root:
`trigger.webhook --route /site-a/map`. Its pages link the way the visitor sees
them (`/map`), as every page already does.

| Request | Answers |
|---|---|
| `a.example.net/map` | `/site-a/map` |
| `a.example.net/api/data` | `/api/data` — shared, because a route says so |
| `a.example.net/_mcp/tools` | `trigger.mcp --route /site-a/tools` |
| `example.org/site-a/map` | 404 — a section's paths answer on its own hosts only |

## Test it locally

Every section has a dev host of its own:
`<root>.<project>.<owner>.localhost` (`site-a.northside.superadmin.localhost`),
listed under Settings → Addressing → Sections. `route_fetch /site-a/map` goes
there by itself.

## What sections share, and what they do not

- Shared: data, credentials, pipelines, error pages, `static`, rooms, map
  tiles, files and the switches. A section that must refuse another's logins
  signs its tokens with its own credential.
- Not shared: paths. A section's paths answer on its hosts (and on the
  platform address, for the Studio) and nowhere else, however the path is
  spelled.
- Login redirects, `next`, `--login` and a published MCP server's connect URL
  are written whole (`/site-a/login`) and sent as the visitor reaches them
  (`/login`).
- Saving addressing refuses a registered route its section cannot reach as
  written, such as `/site-a/api/x` beside a shared `/api/`.
