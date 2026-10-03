# Cookie + JWT Authentication

## What this builds

Login endpoint that verifies credentials against PostgreSQL and issues a JWT in an HttpOnly session cookie. Protected routes use `--auth jwt` to auto-verify the cookie — verified claims land in `input.webhook.auth` right after the trigger (`$trigger.auth` anywhere later). Logout clears the cookie.

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
| `auth_roles` | Roles registered for this credential — populates the **Required Role** checkboxes in webhook nodes. Defines valid values for the JWT `roles` array claim. |
| `auth_redirect` | Where to redirect on 401 (browser navigation only — Sec-Fetch aware) |
| `auth_forbidden_redirect` | Where to redirect on 403 (browser navigation only) |

If `auth_redirect` / `auth_forbidden_redirect` are not set, auth failure returns JSON 401/403 (API behaviour).

---

## Key Concepts

- `trigger.webhook --auth jwt --credential <id>` — auto-verifies JWT from `Authorization: Bearer` header or session cookie. On success: claims in `input.webhook.auth` right after the trigger (`$trigger.auth` anywhere later). On failure: 303 redirect (page nav, to the credential's `auth_redirect`) or 401 JSON (fetch/API).
- `trigger.webhook --role admin --role lecturer` — repeat `--role` once per role; additionally checks the JWT `roles` array claim against the listed roles. Failure: 303 redirect (to `auth_forbidden_redirect`) or 403 JSON. **Empty (no `--role` given) = any valid JWT is accepted — roles are not checked.**
- `auth.token.create --credential <id> --claim "sub={{ input.field }}" --claim "name:public={{ input.name }}" --ttl <duration>` — signs a JWT; output is `token: { access_token, token_type, expires_in, profile }`, read as `{{ input.token.access_token }}`. Claims whose name ends in `:public` are the only ones exposed in the browser via `ctx.auth` — all others remain server-only.
- `web.response.send --header "Set-Cookie=session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"` — sets the session cookie. Quote the whole header — an unquoted `{{ }}` is cut at its first space. The header is sent exactly as written: nothing is added, so the cookie names its own `Path`, `SameSite` and `HttpOnly` (and `Secure` behind HTTPS).
- `web.response.send --status 302 --header "Location=/path"` — issues a 302 redirect.

---

## Pipelines

### POST /auth/login — verify + issue cookie

```
| trigger.webhook --route /auth/login --method POST
| postgres.query.run --credential my-pg --param "1={{ input.webhook.body.identifier }}" \
    -- "SELECT player_id::text, fullname, role FROM app.player WHERE identifier = $1 AND is_active = true"
| logic.if --when "input.query.rows && input.query.rows.length > 0"
(false pin → `web.response.send --status 401 --body "invalid credentials"`)
| javascript.script.run -- "const user = input.query.rows[0]; return { player_id: user.player_id, name: user.fullname, roles: [user.role] };"
| auth.token.create --credential my-jwt --claim "sub={{ input.script.player_id }}" --claim "name:public={{ input.script.name }}" --claim "roles:public={{ input.script.roles }}" --ttl 1d
| web.response.send --status 302 --header "Location=/dashboard" --header "Set-Cookie=session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
```

### GET /dashboard — protected page (auto-verify + redirect)

```
| trigger.webhook --route /dashboard --method GET --auth jwt --credential my-jwt
| postgres.query.run --credential my-pg --param "1={{ $trigger.auth.sub }}" \
    -- "SELECT player_id::text, fullname, email FROM app.player WHERE player_id = $1::uuid"
| javascript.script.run -- "const user = input.query.rows?.[0]; return { user }"
| web.response.send --template pages/dashboard.tsx
```

When JWT is missing/invalid → credential `auth_redirect` fires as a 303 redirect (browser) or 401 JSON (fetch).

### GET /api/me — protected JSON endpoint

```
| trigger.webhook --route /api/me --method GET --auth jwt --credential my-jwt
| javascript.script.run -- "return { ok: true, user: $trigger.auth }"
```

### GET /admin/users — role-gated route

```
| trigger.webhook --route /admin/users --method GET --auth jwt --credential my-jwt --role admin
| postgres.query.run --credential my-pg -- "SELECT player_id::text, fullname, identifier FROM app.player ORDER BY created_at DESC"
| web.response.send --template pages/admin-users.tsx
```

Role mismatch → credential `auth_forbidden_redirect` fires as a 303 redirect (browser) or 403 JSON (fetch).

### POST /auth/logout — clear session cookie

```
| trigger.webhook --route /auth/logout --method POST
| web.response.send --status 302 --header "Location=/auth/login" --header "Set-Cookie=session=; Path=/; Max-Age=0"
```

An empty `value` is allowed and clears the cookie.

---

## Nodes Used

- `trigger.webhook --auth jwt --credential <id>` — auto-verify JWT; `$trigger.auth` = decoded claims
- `trigger.webhook --role <role>` (repeated, one per role) — role check against the credential's `auth_roles`
- `postgres.query.run --credential <id> --param` — look up user by identifier or sub claim, e.g. `--param "1={{ input.webhook.body.identifier }}"` or `--param "1={{ $trigger.auth.sub }}"`
- `auth.token.create --claim "key={{ input.field }}" --ttl <duration>` — sign JWT; output `token: { access_token, token_type, expires_in, profile }`, read as `{{ input.token.access_token }}`. End the claim name with `:public` (e.g. `--claim "name:public={{ input.name }}"`) to expose that claim in the browser via `ctx.auth`. `sub` and other private claims stay server-only.
- `web.response.send --header "Set-Cookie=…"` — set the cookie in the response, sent as written
- `web.response.send --status 302 --header "Location=…"` — redirect
- `web.response.send --template` — protected page template; `input.user` carries auth context

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
