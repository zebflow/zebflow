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
| `auth_roles` | Roles registered for this credential — used to populate the **Required Role** checkboxes in webhook nodes. Defines what values are valid in the JWT `role` claim. |
| `auth_redirect` | Where to redirect on 401 (browser navigation only — `Sec-Fetch-Mode: navigate`) |
| `auth_forbidden_redirect` | Where to redirect on 403 (browser navigation only) |

`auth_redirect` and `auth_forbidden_redirect` trigger only on browser page navigation. API/fetch calls always receive JSON 401/403.

### `--auth-required-role` behaviour

- **One or more roles selected** → JWT `role` claim must match one of the selected roles or request is rejected with 403.
- **No roles selected (empty)** → any holder of a valid JWT may access — role is not checked. Use this for "authenticated but unrestricted" routes.

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
| trigger.webhook --path /auth/login --method GET
| web.response --template pages/auth-login.tsx
```

### auth-login-submit — authenticate and issue token

```
| trigger.webhook --path /auth/login --method POST
| pg.query --credential main-db --params "{{ [input.username] }}" \
    -- "SELECT id::text, username, role FROM users WHERE username = $1 LIMIT 1"
| logic.if --expr "input.rows && input.rows.length > 0"
(false pin → `web.response --status 401 --message "invalid credentials"`)
| script -- "const user = input.rows[0]; return { id: user.id, username: user.username, roles: [user.role] };"
| auth.token.create --credential my-jwt --claim sub={{ input.id }} --claim username={{ input.username }}:public --claim roles={{ input.roles }}:public --expires-in 86400
| web.response --location /dashboard --set-cookie name=session,value={{ input.access_token }},http-only,max-age=86400,path=/
```

> **Note:** `roles` must be an array in the JWT claim — wrap a single DB `role` string with `[user.role]`. If your schema already returns an array (junction table, `text[]` column), use it directly.

### auth-register-page — render register form

```
| trigger.webhook --path /auth/register --method GET
| web.response --template pages/auth-register.tsx
```

### auth-register-submit — create new user

```
| trigger.webhook --path /auth/register --method POST
| logic.if --expr "input.username && input.email && input.password && input.password.length >= 12"
(false pin → `web.response --status 400 --message "username, email and a password of at least 12 characters are required"`)
| script -- "return { username: input.username, email: input.email, role: 'user', input: input.password };"
| crypto --op argon2_hash
(the hash arrives as `input.result` — store that, never the password)

**Never hash a password yourself.** An earlier version of this example wrote
`btoa(password + 'salt')`, which is base64 — not a hash at all, and reversible by
anyone holding the row. `n.crypto` has `argon2_hash` and `argon2_verify`; verify
answers on `true`/`false` pins, so the branch is the check.
| pg.query --credential main-db --params "{{ [input.username, input.email, input.password_hash, input.role] }}" \
    -- "INSERT INTO users (username, email, password_hash, role, created_at) VALUES ($1, $2, $3, $4, NOW()) RETURNING id::text"
| web.response --location /auth/login?registered=1
```

### auth-logout — clear session cookie

```
| trigger.webhook --path /auth/logout --method GET
| web.response --location /auth/login --set-cookie name=session,value=,http-only,max-age=0,path=/
```

### dashboard-protected — JWT-protected page

```
| trigger.webhook --path /dashboard --method GET --auth-type jwt --auth-credential my-jwt
| pg.query --credential main-db --params "{{ input.auth.sub }}" \
    -- "SELECT id::text, username, email, role FROM users WHERE id = $1::uuid"
| script -- "const u = input.rows?.[0]; return { user: u }"
| web.response --template pages/dashboard.tsx
```

JWT missing/invalid → `auth_redirect` fires (browser) or 401 JSON (fetch).

### admin-guard — role-checked admin route

```
| trigger.webhook --path /admin/:section --method GET --auth-type jwt --auth-credential my-jwt --auth-required-role admin
| script -- "return { section: input.params.section, user: input.auth }"
| web.response --template pages/admin-section.tsx
```

Role mismatch → `auth_forbidden_redirect` fires (browser) or 403 JSON (fetch).

---

## Nodes Used

- `trigger.webhook --auth-type jwt --auth-credential <id>` — auto-verify JWT; `input.auth` = decoded claims
- `trigger.webhook --auth-required-role <roles>` — comma-separated roles; checks against JWT `roles` array claim. Empty = any authenticated user.
- `pg.query` — user lookup and insert
- `auth.token.create --claim key={{ input.field }}` — sign JWT; output `{{ input.access_token }}`. Append `:public` to expose that claim in the browser via `ctx.auth` (e.g. `--claim role={{ input.role }}:public`). Private claims like `sub` never reach the browser DOM.
- `web.response --set-cookie` — set HttpOnly session cookie
- `web.response --location` — redirect after login/logout/register
- `web.response --template` — render protected pages

---

## Templates Needed

- `pages/auth-login.tsx` — login form (POST to /auth/login)
- `pages/auth-register.tsx` — register form (POST to /auth/register)
- `pages/dashboard.tsx` — protected user dashboard; receives `input.user`
- `pages/admin-section.tsx` — admin panel; receives `input.section` + `input.user`

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
