# Cookie + JWT Authentication

## What this builds

Login endpoint that verifies credentials against PostgreSQL and issues a JWT in an HttpOnly session cookie. Protected routes use `--auth-type jwt` to auto-verify the cookie — verified claims land in `input.auth`. Logout clears the cookie.

---

## JWT Credential Shape

Create a credential of kind `jwt_signing_key` with these fields in the secret:

```json
{
  "algorithm": "HS256",
  "secret": "your-signing-secret",
  "auth_roles": ["user", "admin", "lecturer", "student"],
  "auth_redirect": "/auth/login",
  "auth_forbidden_redirect": "/home"
}
```

| Field | Purpose |
|---|---|
| `algorithm` | `HS256`, `HS384`, `HS512`, `RS256`, etc. |
| `secret` | Signing key (HS algorithms) |
| `auth_roles` | Roles available in this auth context — used by `--auth-required-role` in the UI |
| `auth_redirect` | Where to redirect on 401 (browser navigation only — Sec-Fetch aware) |
| `auth_forbidden_redirect` | Where to redirect on 403 (browser navigation only) |

If `auth_redirect` / `auth_forbidden_redirect` are not set, auth failure returns JSON 401/403 (API behaviour).

---

## Key Concepts

- `trigger.webhook --auth-type jwt --auth-credential <id>` — auto-verifies JWT from `Authorization: Bearer` header or session cookie. On success: claims in `input.auth`. On failure: 302 redirect (page nav) or 401 JSON (fetch/API).
- `trigger.webhook --auth-required-role admin,lecturer` — additionally checks `input.auth.role` against allowed roles. Failure: 302 redirect or 403 JSON.
- `auth.token.create --credential <id> --claim sub=$.field` — signs a JWT; output is `$.access_token`.
- `web.response --set-cookie name=session,value=$.access_token,http-only,max-age=86400` — sets the session cookie.
- `web.response --location /path` — issues a 302 redirect.

---

## Pipelines

### POST /auth/login — verify + issue cookie

```
| trigger.webhook --path /auth/login --method POST
| pg.query --credential my-pg --params-expr "[input.identifier]" \
    -- "SELECT player_id::text, fullname, role FROM app.player WHERE identifier = $1 AND is_active = true"
| script -- "const user = input.rows?.[0]; if (!user) return { ok: false, error: 'invalid credentials', __status: 401 }; return { player_id: user.player_id, name: user.fullname, role: user.role }"
| auth.token.create --credential my-jwt --claim sub=$.player_id --claim name=$.name --claim role=$.role --expires-in 86400
| web.response --location /dashboard --set-cookie name=session,value=$.access_token,http-only,max-age=86400,path=/
```

### GET /dashboard — protected page (auto-verify + redirect)

```
| trigger.webhook --path /dashboard --method GET --auth-type jwt --auth-credential my-jwt
| pg.query --credential my-pg --params-path /auth/sub \
    -- "SELECT player_id::text, fullname, email FROM app.player WHERE player_id = $1::uuid"
| script -- "const user = input.rows?.[0]; return { user }"
| web.response --template pages/dashboard.tsx
```

When JWT is missing/invalid → credential `auth_redirect` fires (browser) or 401 JSON (fetch).

### GET /api/me — protected JSON endpoint

```
| trigger.webhook --path /api/me --method GET --auth-type jwt --auth-credential my-jwt
| script -- "return { ok: true, user: input.auth }"
```

### GET /admin/users — role-gated route

```
| trigger.webhook --path /admin/users --method GET --auth-type jwt --auth-credential my-jwt --auth-required-role admin
| pg.query --credential my-pg -- "SELECT player_id::text, fullname, identifier FROM app.player ORDER BY created_at DESC"
| web.response --template pages/admin-users.tsx
```

Role mismatch → credential `auth_forbidden_redirect` fires (browser) or 403 JSON (fetch).

### POST /auth/logout — clear session cookie

```
| trigger.webhook --path /auth/logout --method POST
| web.response --location /auth/login --set-cookie name=session,value=,http-only,max-age=0,path=/
```

---

## Nodes Used

- `trigger.webhook --auth-type jwt --auth-credential <id>` — auto-verify JWT; `input.auth` = decoded claims
- `trigger.webhook --auth-required-role <roles>` — role check; comma-separated list from credential `auth_roles`
- `pg.query --params-expr` — look up user by identifier or sub claim
- `auth.token.create --claim key=$.field` — sign JWT; output `$.access_token`
- `web.response --set-cookie` — set HttpOnly cookie in response
- `web.response --location` — redirect
- `web.response --template` — protected page template; `input.user` carries auth context
