# Responses — `web.response`

Everything a pipeline sends back over HTTP goes through `web.response`: JSON,
a rendered page, a redirect, a cookie, a header. Nothing is implicit — a
script cannot set a status or a header; it returns the next payload and the
graph decides which `web.response` answers.

For writing a rendered page to project storage instead of answering the
request, `web.static.generate` renders the same templates
(`help("pipeline/examples/static-entry-generation")`).

## Flags

| Flag | Meaning |
|---|---|
| *(none)* | answer the upstream payload as JSON, status 200 |
| `--template pages/post.tsx` | render the TSX page; the payload becomes its `input`. Path relative to the source root, `.tsx` required |
| `--status 404` | status code (default 200; 302 when `--location` is set) |
| `--location "/x"` | redirect; a `{{ }}` value is resolved from the payload |
| `--body "{{ input.rows }}"` | answer this value instead of the whole payload |
| `--message "text"` | a plain-text body |
| `--set-cookie "spec"` | one cookie, spec below |
| `--header K=V` | extra response header, repeatable |
| `--load-scripts url,url` | external scripts injected into a template response |

Quote any value that contains `{{ }}` or a space as one argument;
`--location {{ input.url }}` unquoted is cut at the first space and refused.

## Cookie spec

```
name=session,value={{ input.token }},http-only,max-age=86400,secure,same-site=Lax,path=/
```

| Part | Default |
|---|---|
| `name=` | required |
| `value=` | literal or `{{ expr }}`; an empty value clears the cookie |
| `http-only` / `no-http-only` | HttpOnly on |
| `secure` | off — turn it on behind HTTPS |
| `max-age=SECS` | 900 |
| `same-site=Lax|Strict|None` | Lax |
| `path=` | `/` |

Logout is `--set-cookie "name=session,value=,max-age=0"`.

## Patterns

**JSON**

```
| trigger.webhook --path /api/posts --method GET
| sekejap.query -- "SELECT id, title FROM posts ORDER BY created_at DESC"
| web.response --body "{{ input.rows }}"
```

**A page**

```
| trigger.webhook --path /blog --method GET
| sekejap.query -- "SELECT id, title, published_at FROM posts ORDER BY published_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx
```

**Found or 404** — branch, then answer on each pin

```
[a] trigger.webhook --path /blog/:slug --method GET
[b] sekejap.query --params "{{ [$trigger.params.slug] }}" -- "SELECT * FROM posts WHERE slug = $1"
[c] logic.if --expr "input.rows.length > 0"
[d] web.response --template pages/post.tsx
[e] web.response --status 404 --template pages/not-found.tsx
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
```

**A designed 404 for paths nobody registered** — a `trigger.weberror`
pipeline, not a webhook. A webhook `--path /*` or `/:path` is not a
catch-all; it never sees a path that matched nothing.

```
| trigger.weberror --code 404
| web.response --status 404 --template pages/not-found.tsx
```

`--code 4xx`, `5xx` or empty widen it; the most specific active one wins.
The payload is `{ error_code, error_message, original_path, method }`.

**Redirect**

```
| trigger.webhook --path /go/signup --method GET
| web.response --location "/auth/register?source=landing"
```

**Redirect to a computed URL**

```
| trigger.webhook --path /after-login --method GET --auth-type jwt --auth-credential jwt_main
| sekejap.query --params "{{ [$trigger.auth.sub] }}" -- "SELECT home FROM users WHERE id = $1"
| web.response --location "{{ input.rows[0]?.home || '/home' }}"
```

**Login — mint a token, set the cookie**

```
[a] trigger.webhook --path /auth/login --method POST
[b] sekejap.query --params "{{ [input.body.email] }}" -- "SELECT id, name, password_hash, roles FROM users WHERE email = $1"
[c] logic.if --expr "input.rows.length === 1"
[d] crypto --op argon2_verify --input "{{ $nodes.a.body.password }}" --hash "{{ input.rows[0].password_hash }}"
[e] script -- "const u = input.rows[0]; return { id: u.id, name: u.name, roles: u.roles || ['member'] }"
[f] auth.token.create --credential jwt_main --claim "sub={{ input.id }}" --claim "name={{ input.name }}:public" --claim "roles={{ input.roles }}:public"
[g] web.response --location /home --set-cookie "name=zebflow_session,value={{ input.access_token }},http-only,max-age=86400,same-site=Lax"
[h] web.response --status 401 --body "{{ { error: 'invalid credentials' } }}"
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [h]
[d]:true -> [e]
[d]:false -> [h]
[e] -> [f]
[f] -> [g]
```

`crypto --op argon2_verify` reads the candidate from `--input` and the stored
hash from `--hash`, routes to `true`/`false`, and passes the payload through
unchanged. The submitted password is no longer in `input` after the query, so
it is read from the trigger's own output, `$nodes.a.body.password`. `roles`
must be an array — a `:public` claim that produces one stays one. The full
recipe with registration: `help("pipeline/examples/cookie-jwt-auth")`.

**Headers**

```
| trigger.webhook --path /api/data --method GET
| sekejap.query -- "SELECT * FROM data"
| web.response --body "{{ input.rows }}" --header Cache-Control=max-age=60 --header X-Version=2
```

## What a template receives

The payload becomes the page's `input`, and `web.response` merges the request
context into it — `route`, `params`, `query`, `search`, `headers`, `auth` — so
they are there even after `sekejap.query` replaced the payload. `input.auth`
carries only the claims minted with `:public`; a token whose claims are all
private gives `input.auth = null` in the browser, while the same claims stay
complete in `$trigger.auth` and `ctx.trigger.auth` server-side.

```tsx
import { useState } from "zeb/react";

export default function Dashboard(input) {
  const user = input.auth;            // { name, roles } — public claims only
  const tab = input.query?.tab ?? "overview";
  if (!user) return <main>Not signed in</main>;
  return <main><h1>Hello {user.name}</h1>{input.rows.map((r) => <p key={r.id}>{r.title}</p>)}</main>;
}
```

`help("web")` for the page side.

## Expressions in flags

`{{ }}` works in every flag value and is resolved just before the response:

```
| web.response --location "/users/{{ $trigger.params.id }}/{{ $nodes.lookup.rows[0].slug }}"
| web.response --header "X-User-Id={{ $trigger.auth.sub }}"
```

Scope: `input`, `$trigger`, `$nodes` — `help("pipeline/dsl")`.
