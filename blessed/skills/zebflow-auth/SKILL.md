---
name: zebflow-auth
description: Login, sessions, roles and protected routes in a Zebflow project — JWT on triggers, cookies, registration, password hashing, OAuth (Google), public vs private claims, public vs private files. Use before building anything a visitor must be signed in for, or anything that must be hidden from one.
license: MIT
metadata:
  version: "1"
---

# Auth: verify at the door

Authentication in Zebflow is a property of the **trigger**, not of a script:
`trigger.webhook --auth-type jwt --auth-credential <id> --auth-required-role admin`
refuses the request before any node runs. Facts:
`help(topic="pipeline/examples/cookie-jwt-auth")`, `pipeline/examples/auth-and-authorization`,
`pipeline/examples/oauth-login-google`, and the `auth.token.*`, `crypto` nodes.

## Pieces

| Piece | What it is |
|---|---|
| a `jwt_signing_key` credential | created by the owner in Studio → Credentials; holds the secret, `auth_redirect` (where a browser goes when refused), `auth_roles`. Its **id** is what `--auth-credential` and `auth.token.create --credential` take (`credential_list`). |
| `auth.token.create` | mints the token from the payload: `--claim "sub={{ input.id }}" --claim "name={{ input.name }}:public" --claim "roles={{ input.roles }}:public"`; answers `{ access_token }` |
| the cookie | `web.response --set-cookie "name=zebflow_session,value={{ input.access_token }},http-only,max-age=86400,same-site=Lax"` — the verifier reads `Authorization: Bearer` first, then this cookie. Behind HTTPS add `secure`. |
| `--auth-required-role` | matches one entry of the token's **`roles` array** claim. A scalar `role` never authorises. |
| `:public` | only claims marked `:public` reach the browser as `input.auth`; everything else stays server-side (`$trigger.auth`, `ctx.trigger.auth`). A public array claim stays an array. |
| `crypto` | `--op argon2_hash --input "{{ input.body.password }}"` → payload plus `result` (`input.body` kept); `--op argon2_verify --input "{{ … }}" --hash "{{ input.rows[0].password_hash }}"` → `true`/`false` pins, payload unchanged |

## Build order

1. **Credential first.** Ask the owner to create the `jwt_signing_key` (with
   `auth_redirect: /login`); `credential_list` gives you its id. You cannot
   create credentials over MCP, and a guessed id is an auth failure on every
   request.
2. **Registration.** `POST /auth/register`: validate `input.body`, hash the
   password, insert, redirect to `/login`:

   ```
   | trigger.webhook --path /auth/register --method POST
   | logic.if --expr "typeof input.body?.email === 'string' && typeof input.body?.password === 'string' && input.body.password.length >= 12"
   | crypto --op argon2_hash --input "{{ input.body.password }}"
   | sekejap.query --read-only false --params "{{ [input.body.email, input.result, ['user'], new Date().toISOString()] }}" -- "INSERT INTO users (email, password_hash, roles, created_at) VALUES ($1, $2, $3, $4)"
   | web.response --location /login?registered=1
   ```

   The `users` table gets its id from `_key TEXT PRIMARY KEY DEFAULT UUIDV4()`;
   "email is unique" is a `SELECT` before the `INSERT`, not a constraint
   (`zebflow-data`).
3. **Login.** `POST /auth/login`: look the user up by `input.body.email`,
   `logic.if` one row, `crypto --op argon2_verify` with `--input "{{ $nodes.<trigger id>.body.password }}"`
   and `--hash "{{ input.rows[0].password_hash }}"`, mint the token with
   `roles` as an array, set the cookie, `--location /home`. The `false` pins
   answer `401` — never a different message for "no such user" and "wrong
   password".
4. **Logout.** `web.response --location /login --set-cookie "name=zebflow_session,value=,max-age=0"`
   (an empty value clears the cookie).
5. **Protect.** Put `--auth-type jwt --auth-credential <id>` on every route
   that needs a user and `--auth-required-role` on every route that needs a
   role — pages **and** their POST/API pipelines. A protected page whose API
   is open is open.
6. **Use identity server-side.** `$trigger.auth.sub` in `--params`,
   `ctx.trigger.auth.sub` in scripts; never trust an id from `input.body`.
7. **OAuth** (Google) is the same shape with `http.request` for the token
   exchange and `kv.set/kv.get` for the state parameter —
   `help(topic="pipeline/examples/oauth-login-google")`.

## Files

Objects under `public/` are served anonymously at `/files/{owner}/{project}/…`;
everything else is private (`/fs/…`, session required). Upload with
`fs.save --folder public/uploads` only when the file is meant for everyone.

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
- a password compared in a script instead of `crypto --op argon2_verify`;
- a secret, token or hash marked `:public` or written to a page;
- a credential id or secret typed into a pipeline body from memory.
