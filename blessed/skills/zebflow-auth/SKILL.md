---
name: zebflow-auth
description: Login, sessions, roles and protected routes in a Zebflow project — JWT on triggers, cookies, registration, password hashing, OAuth (Google; signing agents in to a published MCP server), public vs private claims, public vs private files. Use before building anything a visitor must be signed in for, or anything that must be hidden from one.
license: MIT
metadata:
  version: "1"
---

# Auth: verify at the door

Authentication in Zebflow is a property of the **trigger**, not of a script:
`trigger.webhook --auth jwt --credential <id> --role admin`
refuses the request before any node runs. Facts:
`help(topic="pipeline/examples/cookie-jwt-auth")`, `pipeline/examples/auth-and-authorization`,
`pipeline/examples/oauth-login-google`, and the `auth.token.*`, `crypto.*` nodes.

## Pieces

| Piece | What it is |
|---|---|
| a `jwt_signing_key` credential | created by the owner in Studio → Credentials; holds the secret, `auth_redirect` (where a browser goes when refused), `auth_roles`. Its **id** (`credential_list`) is what `trigger.webhook --credential` and `auth.token.create --credential` take. |
| `auth.token.create` | mints the token from `--claim` values (repeated `key=value`; a name ending in `:public` reaches the browser). It adds `token`, so the next node reads `input.token.access_token`. |
| the cookie | `web.response.send --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"` — sent exactly as written, so the attributes are yours to write; behind HTTPS add `; Secure`. The verifier reads `Authorization: Bearer` first, then this cookie. |
| roles | `trigger.webhook --role admin` (repeat for several) matches one entry of the token's **`roles` array** claim. A scalar `role` never authorises. |
| `:public` claims | only claims marked `:public` reach the browser, as a page's `input.auth`; everything else stays server-side (`$trigger.auth`, `ctx.trigger.auth`). A public array claim stays an array. |
| `crypto.password.hash` / `crypto.password.verify` | hash on registration, verify on login; both take the password with `--from`, both add `password` and keep the payload, so `input.webhook.body` is still there. Verify routes on `true` / `false`; an empty `--hash` (no such user) goes to `:error`. |

Every node above has its page: `help(topic="pipeline/nodes/auth.token.create")`,
`pipeline/nodes/crypto.password.verify`.

## Build order

1. **Credential first.** Ask the owner to create the `jwt_signing_key` (with
   `auth_redirect: /login`); `credential_list` gives you its id. You cannot
   create credentials over MCP, and a guessed id is an auth failure on every
   request.
2. **Registration.** `POST /auth/register`: validate `$trigger.body`, hash the
   password, insert, redirect to `/login`:

   ```
   | trigger.webhook --route /auth/register --method POST
   | logic.if --when "typeof $trigger.body?.email === 'string' && typeof $trigger.body?.password === 'string' && $trigger.body.password.length >= 12"
   | crypto.password.hash --from "{{ $trigger.body.password }}"
   | sekejap.query.run --write --param "1={{ $trigger.body.email }}" --param "2={{ input.password.hash }}" --param "3={{ ['user'] }}" --param "4={{ new Date().toISOString() }}" -- "INSERT INTO users (email, password_hash, roles, created_at) VALUES ($1, $2, $3, $4)"
   | web.response.send --status 302 --header "Location=/login?registered=1"
   ```

   The `users` table gets its id from `_key TEXT PRIMARY KEY DEFAULT UUIDV4()`;
   "email is unique" is a `SELECT` before the `INSERT`, not a constraint
   (`zebflow-data`).
3. **Login.** `POST /auth/login`: look the user up by `$trigger.body.email`,
   `logic.if` one row, `crypto.password.verify --from "{{ $trigger.body.password }}" --hash "{{ input.query.rows[0]?.password_hash }}"`,
   mint the token with `roles` as an array, set the cookie,
   `--status 303 --header "Location=/home"`. The `false` pins answer `401` —
   never a different message for "no such user" and "wrong password":

   ```
   [a] trigger.webhook --route /auth/login --method POST
   [b] sekejap.query.run --param "1={{ $trigger.body.email }}" -- "SELECT _key, password_hash, roles FROM users WHERE email = $1"
   [c] logic.if --when "input.query.rows.length === 1"
   [d] crypto.password.verify --from "{{ $trigger.body.password }}" --hash "{{ input.query.rows[0].password_hash }}"
   [e] auth.token.create --credential jwt_main --ttl 1d --claim "sub={{ input.query.rows[0]._key }}" --claim "roles:public={{ input.query.rows[0].roles }}"
   [f] web.response.send --status 303 --header "Location=/home" --header "Set-Cookie=zebflow_session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
   [g] web.response.send --status 401 --body "{{ { error: 'wrong email or password' } }}"
   [a] -> [b]
   [b] -> [c]
   [c]:true -> [d]
   [d]:true -> [e]
   [e] -> [f]
   [c]:false -> [g]
   [d]:false -> [g]
   ```
4. **Logout.** `web.response.send --status 302 --header "Location=/login" --header "Set-Cookie=zebflow_session=; Path=/; Max-Age=0"`
   (an empty value clears the cookie).
5. **Protect.** Put `--auth jwt --credential <id>` on every route
   that needs a user and `--role` on every route that needs a
   role — pages **and** their POST/API pipelines. A protected page whose API
   is open is open.
6. **Use identity server-side.** `$trigger.auth.sub` in `--param`,
   `ctx.trigger.auth.sub` in scripts; never trust an id from `$trigger.body`.
7. **OAuth** (Google) is the same shape with `http.response.fetch` for the token
   exchange and `kv.entry.put`/`kv.entry.get` for the state parameter —
   `help(topic="pipeline/examples/oauth-login-google")`.
8. **Signing agents in to a published MCP server** (ChatGPT, claude.ai
   connectors) is the other direction — the app is the OAuth server, its
   login page the sign-in: `trigger.mcp --auth oauth --credential <jwt key>
   --login /auth/login`, and the login pipeline ends in
   `auth.oauth.approve --ticket … --token "{{ input.token.access_token }}"
   --client … --redirect-host …` instead of setting a cookie. The page shows
   `client` and `redirect_host` from its query and posts them back unchanged —
   `help(topic="pipeline/examples/mcp-oauth-login")`. Keys under `zf.` are the
   platform's; no `kv.*` node takes one.

## Files

Every stored object is private until the owner exposes its folder in Studio →
Files; a folder name (`public/`) means nothing and no node can expose a file.
A session cookie never opens a private file outside the Studio.

## Prove it — every row, every time

| Request | Expected |
|---|---|
| browser `GET /admin` without a cookie | `303` to `auth_redirect` |
| `fetch("/admin/api")` without a token | `401` JSON |
| a token whose `roles` lacks the required role | `403` |
| a valid token | the page, with `input.auth` holding only the `:public` claims |
| `POST /auth/login` with a wrong password | `401`, no cookie set |
| `POST /auth/login` correct | `303` + `Set-Cookie` with `HttpOnly` (and `Secure` behind HTTPS) |
| `POST /auth/logout` | `Set-Cookie … Max-Age=0`; the next `GET /admin` redirects |

Use curl with a cookie jar for the browser cases and `-H "Authorization: Bearer …"`
for the API cases; `pipeline_get_invocations` shows which node refused.

## Never

- a role check in a script instead of on the trigger;
- a password compared in a script instead of `crypto.password.verify`;
- a secret, token or hash marked `:public` or written to a page;
- a credential id or secret typed into a pipeline body from memory.
