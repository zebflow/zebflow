# Auth and Authorization

## What this builds

Full JWT-based authentication: login page, register, session validation, role-based access control, protected routes, and logout. Works with PostgreSQL or Sekejap for user storage.

---

## JWT Credential

Create a `jwt_signing_key` credential in the Credentials UI. Fields:

```json
{
  "algorithm": "HS256",
  "secret": "your-signing-secret",
  "auth_roles": ["user", "admin"],
  "auth_redirect": "/auth/login",
  "auth_forbidden_redirect": "/home"
}
```

| Field | Purpose |
|---|---|
| `algorithm` | `HS256` / `HS384` / `HS512` (symmetric) or `RS256` / `RS384` / `RS512` / `ES256` / `ES384` (asymmetric) |
| `secret` | Signing key for HS* algorithms |
| `auth_roles` | Roles registered for this credential — used to populate the **Required Role** checkboxes in webhook nodes. Defines what values are valid in the JWT `roles` array claim. |
| `auth_redirect` | Where to redirect on 401 (browser navigation only — `Sec-Fetch-Mode: navigate`) |
| `auth_forbidden_redirect` | Where to redirect on 403 (browser navigation only) |

`auth_redirect` and `auth_forbidden_redirect` trigger only on browser page navigation. API/fetch calls always receive JSON 401/403.

### `--role` behaviour

- **One or more `--role` flags** (repeated, one per role) → the JWT `roles` array claim must contain one of the listed roles or the request is rejected with 403.
- **No `--role` given (empty)** → any holder of a valid JWT may access — role is not checked. Use this for "authenticated but unrestricted" routes.

---

## Pipelines

1. `GET /auth/login` → render login page
2. `POST /auth/login` → validate credentials → issue JWT cookie → redirect
3. `GET /auth/register` → render register page
4. `POST /auth/register` → hash password → create user → redirect
5. `GET /auth/logout` → clear cookie → redirect to login
6. `GET /dashboard` → JWT auto-verify → load user → render protected page
7. `GET /admin/*` → JWT auto-verify + role check → serve or redirect/403

---

### auth-login-page — render login form

```
| trigger.webhook --route /auth/login --method GET
| web.response.send --template pages/auth-login.tsx
```

### auth-login-submit — authenticate and issue token

```
| trigger.webhook --route /auth/login --method POST
| pg.query.run --credential main-db --param "1={{ input.webhook.body.username }}" \
    -- "SELECT id::text, username, role FROM users WHERE username = $1 LIMIT 1"
| logic.if --expr "input.query.rows && input.query.rows.length > 0"
(false pin → `web.response.send --status 401 --body "invalid credentials"`)
| script.result.run -- "const user = input.query.rows[0]; return { id: user.id, username: user.username, roles: [user.role] };"
| auth.token.create --credential my-jwt --claim "sub={{ input.id }}" --claim "username:public={{ input.username }}" --claim "roles:public={{ input.roles }}" --expires-in 86400
| web.response.send --status 302 --header "Location=/dashboard" --header "Set-Cookie=session={{ input.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
```

> **Note:** `roles` must be an array in the JWT claim — wrap a single DB `role` string with `[user.role]`. If your schema already returns an array (junction table, `text[]` column), use it directly.

### auth-register-page — render register form

```
| trigger.webhook --route /auth/register --method GET
| web.response.send --template pages/auth-register.tsx
```

### auth-register-submit — create new user

```
| trigger.webhook --route /auth/register --method POST
| logic.if --expr "input.webhook.body.username && input.webhook.body.email && input.webhook.body.password && input.webhook.body.password.length >= 12"
(false pin → `web.response.send --status 400 --body "username, email and a password of at least 12 characters are required"`)
| crypto.password.hash --from "{{ $trigger.body.password }}"
(`crypto.password.hash` adds `password: { hash, algorithm }` to the payload and
keeps everything else, so `$trigger.body.username` is still reachable for the insert.)
| pg.query.run --credential main-db --write --param "1={{ $trigger.body.username }}" --param "2={{ $trigger.body.email }}" --param "3={{ input.password.hash }}" --param "4=user" \
    -- "INSERT INTO users (username, email, password_hash, role, created_at) VALUES ($1, $2, $3, $4, NOW()) RETURNING id::text"
| web.response.send --status 302 --header "Location=/auth/login?registered=1"
```

**Never hash a password yourself.** An earlier version of this example wrote
`btoa(password + 'salt')`, which is base64 — not a hash at all, and reversible by
anyone holding the row. `crypto.password.hash` and `crypto.password.verify` do it;
verify answers on `true`/`false` pins, so the branch is the check.

### auth-logout — clear session cookie

```
| trigger.webhook --route /auth/logout --method GET
| web.response.send --status 302 --header "Location=/auth/login" --header "Set-Cookie=session=; Path=/; Max-Age=0"
```

### dashboard-protected — JWT-protected page

```
| trigger.webhook --route /dashboard --method GET --auth jwt --credential my-jwt
| pg.query.run --credential main-db --param "1={{ $trigger.auth.sub }}" \
    -- "SELECT id::text, username, email, role FROM users WHERE id = $1::uuid"
| script.result.run -- "const u = input.query.rows?.[0]; return { user: u }"
| web.response.send --template pages/dashboard.tsx
```

JWT missing/invalid → `auth_redirect` fires as a 303 redirect (browser navigation) or 401 JSON (fetch/API).

### admin-guard — role-checked admin route

```
| trigger.webhook --route /admin/:section --method GET --auth jwt --credential my-jwt --role admin
| script.result.run -- "return { section: input.webhook.params.section, user: $trigger.auth }"
| web.response.send --template pages/admin-section.tsx
```

Role mismatch → `auth_forbidden_redirect` fires as a 303 redirect (browser navigation) or 403 JSON (fetch/API).

---

## Nodes Used

- `trigger.webhook --auth jwt --credential <id>` — auto-verify JWT; `$trigger.auth` = decoded claims
- `trigger.webhook --role <role>` (repeated, one per role) — checks against JWT `roles` array claim. Empty = any authenticated user.
- `pg.query.run` — user lookup and insert
- `auth.token.create --claim "key={{ input.field }}"` — sign JWT; output `{{ input.access_token }}`. End the claim name with `:public` to expose that claim in the browser via `ctx.auth` (e.g. `--claim "role:public={{ input.role }}"`). Private claims like `sub` never reach the browser DOM.
- `web.response.send --header "Set-Cookie=…"` — set the session cookie, sent as written: write `Path=/; SameSite=Lax; HttpOnly` (and `Secure` behind HTTPS)
- `web.response.send --status 302 --header "Location=…"` — redirect after login/logout/register
- `web.response.send --template` — render protected pages

---

## Templates Needed

- `pages/auth-login.tsx` — login form (POST to /auth/login)
- `pages/auth-register.tsx` — register form (POST to /auth/register)
- `pages/dashboard.tsx` — protected user dashboard; receives `input.user`
- `pages/admin-section.tsx` — admin panel; receives `input.section` + `input.user`

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
