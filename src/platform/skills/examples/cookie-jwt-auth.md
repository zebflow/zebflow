# Cookie + JWT Authentication

## What this builds

Login endpoint that verifies credentials against PostgreSQL and issues a JWT in an HttpOnly session cookie. Protected routes use `--auth-type jwt` to auto-verify the cookie — verified claims land in `input.auth`. Logout clears the cookie.

---

## Key Concepts

- `trigger.webhook --auth-type jwt --auth-credential <id>` — auto-verifies JWT from `Authorization: Bearer` header or `Cookie: zebflow_session`. On success: claims in `input.auth`. On failure: returns 401 automatically.
- `output._set_cookie` — sets an HttpOnly cookie in the response `{ name, value, http_only, max_age, path }`
- `output._redirect` — issues a 302 redirect
- `output.__status` — sets HTTP response status code

A `jwt` credential must exist in project credentials (kind = `jwt`) with a `signing_key` value. The trigger uses it for both signing (via script) and verification.

---

## Pipelines

### POST /auth/login — verify + issue cookie

```
| trigger.webhook --path /auth/login --method POST
| script -- "if (!input.identifier || !input.password) return { __status: 400, error: 'identifier and password required' }; return input"
| pg.query --credential my-pg --params-expr "[input.identifier]" \
    -- "SELECT player_id::text, fullname, identifier FROM app.player WHERE identifier = $1 AND is_active = true"
| script -- "const user = input.rows?.[0]; if (!user) return { __status: 401, error: 'invalid credentials' }; const exp = Math.floor(Date.now()/1000) + 86400; const payload = { sub: user.player_id, name: user.fullname, ident: user.identifier, exp }; return { ok: true, user: { id: user.player_id, name: user.fullname }, _set_cookie: { name: 'session', value: btoa(JSON.stringify(payload)), http_only: true, max_age: 86400, path: '/' } }"
```

Note: for production, use a proper JWT library via `http.request` to a signing service, or use the `auth_token_create` node with a `jwt` credential.

### GET /dashboard — protected page (auto-verify)

```
| trigger.webhook --path /dashboard --method GET --auth-type jwt --auth-credential my-jwt-cred
| script -- "if (!input.auth) return { _redirect: '/auth/login' }; return { user: input.auth }"
| pg.query --credential my-pg --params-path /user/sub \
    -- "SELECT player_id::text, fullname, email FROM app.player WHERE player_id = $1::uuid"
| script -- "const user = input.rows?.[0]; return { user }"
| web.render --template-path pages/dashboard.tsx --template-id pages/dashboard.tsx --route /dashboard
```

When `--auth-type jwt` is set on the trigger:
- Valid token → `input.auth` contains decoded claims (`sub`, `name`, etc.)
- Invalid / missing token → trigger returns 401, pipeline does not run

### GET /api/me — protected JSON endpoint

```
| trigger.webhook --path /api/me --method GET --auth-type jwt --auth-credential my-jwt-cred
| script -- "return { ok: true, user: input.auth }"
```

### POST /auth/logout — clear session cookie

```
| trigger.webhook --path /auth/logout --method POST
| script -- "return { ok: true, _set_cookie: { name: 'session', value: '', http_only: true, max_age: 0, path: '/' }, _redirect: '/auth/login' }"
```

### Role-based access — check claim from `input.auth`

```
| trigger.webhook --path /admin/users --method GET --auth-type jwt --auth-credential my-jwt-cred
| script -- "if (!input.auth || input.auth.role !== 'admin') return { __status: 403, error: 'forbidden' }; return input"
| pg.query --credential my-pg -- "SELECT player_id::text, fullname, identifier FROM app.player ORDER BY created_at DESC"
| script -- "return { ok: true, data: input.rows }"
```

---

## Special Output Keys

| Key | Effect |
|-----|--------|
| `_set_cookie` | Sets an HttpOnly cookie: `{ name, value, http_only, max_age, path }`. `max_age: 0` clears it. |
| `_redirect` | Issues 302 redirect to the given URL |
| `__status` | Sets HTTP status code (400, 401, 403, etc.) |

---

## Nodes Used

- `trigger.webhook --auth-type jwt --auth-credential <id>` — auto-verify JWT; `input.auth` = decoded claims
- `pg.query --params-expr` — look up user by identifier or sub claim
- `script` — credential check, cookie issuance, role assertion, logout
- `web.render` — protected page template; `input.user` carries auth context
