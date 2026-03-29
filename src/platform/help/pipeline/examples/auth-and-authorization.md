# Auth and Authorization

## What this builds

Full JWT-based authentication: login page, register, session validation, role-based access control, protected routes, and logout. Works with PostgreSQL or Sekejap for user storage.

---

## JWT Credential

Create a `jwt_signing_key` credential with RBAC config in the secret:

```json
{
  "algorithm": "HS256",
  "secret": "your-signing-secret",
  "auth_roles": ["user", "admin"],
  "auth_redirect": "/auth/login",
  "auth_forbidden_redirect": "/home"
}
```

`auth_redirect` and `auth_forbidden_redirect` trigger only on browser page navigation (Sec-Fetch-Mode: navigate). API/fetch calls always receive JSON 401/403.

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
| pg.query --credential main-db --params-expr "[input.body.username]" \
    -- "SELECT id::text, username, role FROM users WHERE username = $1 LIMIT 1"
| script -- "const user = input.rows?.[0]; if (!user) return { ok: false, error: 'invalid credentials', __status: 401 }; return { id: user.id, username: user.username, role: user.role }"
| auth.token.create --credential my-jwt --claim sub=$.id --claim username=$.username --claim role=$.role --expires-in 86400
| web.response --location /dashboard --set-cookie name=session,value=$.access_token,http-only,max-age=86400,path=/
```

### auth-register-page — render register form

```
| trigger.webhook --path /auth/register --method GET
| web.response --template pages/auth-register.tsx
```

### auth-register-submit — create new user

```
| trigger.webhook --path /auth/register --method POST
| script -- "const { username, email, password } = input.body; if (!username || !email || !password) return { error: 'all fields required', __status: 400 }; if (password.length < 8) return { error: 'password too short', __status: 400 }; return { username, email, password_hash: btoa(password + 'salt'), role: 'user' }"
| pg.query --credential main-db --params-expr "[input.username, input.email, input.password_hash, input.role]" \
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
| pg.query --credential main-db --params-path /auth/sub \
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
- `trigger.webhook --auth-required-role <roles>` — comma-separated roles from credential `auth_roles`
- `pg.query` — user lookup and insert
- `auth.token.create --claim key=$.field` — sign JWT; output `$.access_token`
- `web.response --set-cookie` — set HttpOnly session cookie
- `web.response --location` — redirect after login/logout/register
- `web.response --template` — render protected pages

---

## Templates Needed

- `pages/auth-login.tsx` — login form (POST to /auth/login)
- `pages/auth-register.tsx` — register form (POST to /auth/register)
- `pages/dashboard.tsx` — protected user dashboard; receives `input.user`
- `pages/admin-section.tsx` — admin panel; receives `input.section` + `input.user`
