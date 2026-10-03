# Responses — `web.response.send`

Everything a pipeline sends back over HTTP goes through `web.response.send`: JSON,
a rendered page, a redirect, a cookie, a header. Nothing is implicit — a
script cannot set a status or a header; it returns the next payload and the
graph decides which `web.response.send` answers.

For writing a rendered page to project storage instead of answering the
request, `web.site.generate` renders the same templates
(`help("pipeline/examples/static-entry-generation")`).

## Flags

The table is rendered from the node's definition when this page is read, so
it cannot lag behind the code; `help("pipeline/nodes/web.response")` has the
same rows with their schemas and examples.

<!-- node-flags:web.response.send -->

Quote any value that contains `{{ }}` or a space as one argument;
`--header Location={{ input.url }}` unquoted is cut at the first space and refused.

## Redirects, cookies, bodies

A redirect is a 3xx `--status` with a `Location` header; a `Location` without
a 3xx, or a 3xx (other than 304) without a `Location`, is refused at run.

```
web.response.send --status 303 --header "Location=/home"
```

A cookie is a `Set-Cookie` header, sent exactly as written — nothing is added,
so a session cookie writes its own attributes. Repeat `--header` for several;
a repeated name is sent once per value, so two `Set-Cookie` headers are two
cookies:

```
web.response.send --status 303 --header "Location=/home" \
  --header "Set-Cookie=session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly" \
  --header "Set-Cookie=theme=dark; Path=/; Max-Age=31536000; SameSite=Lax"
```

A session cookie carries `Path=/; SameSite=Lax; HttpOnly`, and `Secure`
behind HTTPS. A cookie without `Max-Age` or `Expires` lasts until the browser
closes. Logout is `--header "Set-Cookie=session=; Path=/; Max-Age=0"` — the
same `Path` as the cookie it clears.

`--body` answers a value: a string as `text/plain`, anything else as JSON,
unless a `Content-Type` header says otherwise. At most one of `--body`,
`--template` and `--file`; with none, the payload answers as JSON.
`web.response.send` passes its payload on unchanged — it adds no key.

## Patterns

**JSON**

```
| trigger.webhook --route /api/posts --method GET
| sekejap.query.run -- "SELECT id, title FROM posts ORDER BY created_at DESC"
| web.response.send --body "{{ input.query.rows }}"
```

**A page**

```
| trigger.webhook --route /blog --method GET
| sekejap.query.run -- "SELECT id, title, published_at FROM posts ORDER BY published_at DESC LIMIT 20"
| web.response.send --template pages/blog-home.tsx
```

**Found or 404** — branch, then answer on each pin

```
[a] trigger.webhook --route /blog/:slug --method GET
[b] sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT * FROM posts WHERE slug = $1"
[c] logic.if --expr "input.query.rows.length > 0"
[d] web.response.send --template pages/post.tsx
[e] web.response.send --status 404 --template pages/not-found.tsx
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
```

**A designed 404 for paths nobody registered** — a `trigger.error`
pipeline, not a webhook. A webhook `--route /*` or `/:path` is not a
catch-all; it never sees a path that matched nothing.

```
| trigger.error --status 404
| web.response.send --status 404 --template pages/not-found.tsx
```

`--status 4xx`, `5xx` or empty widen it; the most specific active one wins.
The trigger answers under `error`: `input.error.error_code`,
`input.error.error_message`, `input.error.original_path`, `input.error.path`,
`input.error.method` (`$trigger.error_code` … anywhere later in the chain).

**Redirect**

```
| trigger.webhook --route /go/signup --method GET
| web.response.send --status 302 --header "Location=/auth/register?source=landing"
```

**Redirect to a computed URL**

```
| trigger.webhook --route /after-login --method GET --auth jwt --credential jwt_main
| sekejap.query.run --param "1={{ $trigger.auth.sub }}" -- "SELECT home FROM users WHERE id = $1"
| web.response.send --status 302 --header "Location={{ input.query.rows[0]?.home || '/home' }}"
```

**Login — mint a token, set the cookie**

```
[a] trigger.webhook --route /auth/login --method POST
[b] sekejap.query.run --param "1={{ input.webhook.body.email }}" -- "SELECT id, name, password_hash, roles FROM users WHERE email = $1"
[c] logic.if --expr "input.query.rows.length === 1"
[d] crypto.password.verify --from "{{ $nodes.a.webhook.body.password }}" --hash "{{ input.query.rows[0]?.password_hash }}"
[e] javascript.script.run -- "const u = input.query.rows[0]; return { id: u.id, name: u.name, roles: u.roles || ['member'] }"
[f] auth.token.create --credential jwt_main --claim "sub={{ input.script.id }}" --claim "name:public={{ input.script.name }}" --claim "roles:public={{ input.script.roles }}"
[g] web.response.send --status 302 --header "Location=/home" --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
[h] web.response.send --status 401 --body "{{ { error: 'invalid credentials' } }}"
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [h]
[d]:true -> [e]
[d]:false -> [h]
[e] -> [f]
[f] -> [g]
```

`crypto.password.verify` reads the candidate from `--from` and the stored
hash from `--hash`, routes to `true`/`false`, and adds `password: { valid }`
to the payload, keeping `input.query.rows` for `[e]`. An empty hash is
refused to `:error`, never `true`. The submitted password is no longer in `input` after the query, so
it is read from the trigger's own output, `$nodes.a.webhook.body.password`. `roles`
must be an array. `:public` goes on the claim's name (`roles:public=`), so
the value is a whole `{{ }}` and keeps the type its expression gives: a list
stays a list, an 18-digit NIM or NIP stays the text it was. The full
recipe with registration: `help("pipeline/examples/cookie-jwt-auth")`.

**Headers**

```
| trigger.webhook --route /api/data --method GET
| sekejap.query.run -- "SELECT * FROM data"
| web.response.send --body "{{ input.query.rows }}" --header Cache-Control=max-age=60 --header X-Version=2
```

## What a template receives

The payload becomes the page's `input`, and `web.response.send` merges the request
context into it — `route`, `params`, `query`, `search`, `headers`, `auth` — so
they are there even after `sekejap.query.run` replaced the payload. `input.auth`
carries only the claims minted with `:public`; a token whose claims are all
private gives `input.auth = null` in the browser, while the same claims stay
complete in `$trigger.auth` and `ctx.trigger.auth` server-side.

```tsx
import { useState } from "zeb/react";

export default function Dashboard(input) {
  const user = input.auth;            // { name, roles } — public claims only
  const tab = input.query?.tab ?? "overview";
  if (!user) return <main>Not signed in</main>;
  return <main><h1>Hello {user.name}</h1>{input.query.rows.map((r) => <p key={r.id}>{r.title}</p>)}</main>;
}
```

`help("web")` for the page side.

## Expressions in flags

`{{ }}` works in every flag value and is resolved just before the response:

```
| web.response.send --status 302 --header "Location=/users/{{ $trigger.params.id }}/{{ $nodes.lookup.query.rows[0].slug }}"
| web.response.send --header "X-User-Id={{ $trigger.auth.sub }}"
```

Scope: `input`, `$trigger`, `$nodes` — `help("pipeline/dsl")`.

## Project files and an installable app — `--file`

`web.response.send --file` answers a project file: content type by extension, a
`.ts` compiled to JavaScript, everything else byte for byte. That is how a
robots.txt, a sitemap, an icon or a web-app manifest is served, and how a
service worker is: every script served this way starts with
`self.__ZF = { version, source }` (the build and the file's hash) so a
worker can key its cache on them.

```text
register pipelines/pwa/manifest -- | trigger.webhook --route /manifest.webmanifest --method GET | web.response.send --file pwa/manifest.webmanifest
register pipelines/pwa/worker   -- | trigger.webhook --route /sw.js --method GET               | web.response.send --file pwa/site.sw.ts
register pipelines/pwa/icons    -- | trigger.webhook --route /pwa/{file} --method GET          | web.response.send --root pwa/icons --file "{{ input.webhook.params.file }}"
```

- The manifest is a JSON file you write (`name`, `id`, `start_url`, `scope`
  ending in `/`, `display: standalone`, `icons` at 192 and 512). A second app
  (a member area at `/member/`) is a second file and route.
- A worker controls only pages under the directory it is served from, so the
  site's worker answers at `/sw.js`; from deeper, add
  `--header Service-Worker-Allowed=/`. A stored object on the file host
  would control nothing — that host never runs scripts.
- With `--root`, `--file` is a bare filename, usually from the route, and can
  never leave that folder — safe to expose.
- Every page carries the manifest and the Apple icon in `page.head.links` and
  its colour in `page.head.themeColor`. A component in the shell registers the
  worker once (`navigator.serviceWorker.register("/sw.js")`) and shows the
  install offer as a quiet card, never a modal.
- Check it: `route_fetch path="/"` answers a `pwa` block — `installable: true`
  or the `reasons` that stop it. Localhost is a secure origin, so all of this
  works on the dev host; only the install button itself needs a real
  (non-headless) Chrome.
