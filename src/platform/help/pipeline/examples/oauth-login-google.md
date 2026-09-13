# Sign in with Google (OAuth login)

## What this builds

A "Sign in with Google" button for a members-only site. The visitor is sent to
Google, comes back with an authorization code, the pipeline exchanges that code
for the visitor's identity, checks that the e-mail is an accepted member, and
issues the same HttpOnly session cookie that
`help("pipeline/examples/cookie-jwt-auth")` describes. Every protected route
after that is plain `--auth-type jwt`; nothing downstream knows Google was
involved.

Two pipelines, two credentials, one table.

---

## Which credential — `secure_request`, not `oauth2`

Zebflow has two credential kinds with "OAuth" in their story. They solve
different problems, and picking the wrong one fails at run time with
`PLATFORM_OAUTH2_REFRESH: no refresh_token stored`.

| | `secure_request` | `oauth2` |
|---|---|---|
| Who is authorizing | **Each visitor**, for themselves | **The project**, once, as itself |
| What is stored | The client secret and a request template | One `refresh_token`, obtained by an admin pressing **Authorize** in the Credentials UI |
| What the node does | Renders the template with the visitor's `code` and sends it | Attaches `Authorization: Bearer <access_token>` to whatever `--url` you gave it, refreshing the token when it expires |
| Use it for | **Login** — "Sign in with Google/GitHub/…" | Calling a provider API as the project — read a Google Sheet, post to a GitHub repo |

Login is the left column. A visitor's code is exchanged once and discarded; the
credential never stores a token because there is no single identity to store.

---

## Credential 1 — `google-token-exchange` (kind `secure_request`)

The token exchange is a POST to Google with the client secret in the body. The
secret belongs in the credential; the pipeline only supplies the visitor's code.

```json
{
  "request": {
    "method": "POST",
    "url": "https://oauth2.googleapis.com/token",
    "headers": { "Content-Type": "application/x-www-form-urlencoded" },
    "body": "code=<CODE>&client_id=YOUR_CLIENT_ID.apps.googleusercontent.com&client_secret=<CLIENT_SECRET>&redirect_uri=https://your.site/wh/OWNER/PROJECT/auth/google/callback&grant_type=authorization_code"
  },
  "variables": [
    { "name": "CODE", "required": true }
  ],
  "secrets": {
    "CLIENT_SECRET": "GOCSPX-…"
  }
}
```

| Part | What it is |
|---|---|
| `request.*` | URL, method, headers and body templates. `<NAME>` placeholders are filled from `secrets` and `variables`. |
| `variables` | Values the **pipeline** supplies at run time. Each becomes a `--bind NAME=<expr>` on `http.request`; `required: true` fails the node if the binding is missing. |
| `secrets` | Values the **credential** supplies. Never in a pipeline, never in a trace — every secret value is redacted wherever it would appear. |
| `egress` | Leave blank for a public provider. Outbound HTTP already refuses private and loopback addresses; `egress.allow_private` plus `egress.allowed_hosts` is the exception list for a provider on your own network. `allowed_paths` / `allowed_methods` pin the resolved path and method when the template lets a variable vary them. |

The Credentials UI has an editor for this kind (Request Method, URL Template,
Header Templates, Body Template, Secrets, Runtime Variables, Egress).
`redirect_uri` must match, byte for byte, the one you registered in Google Cloud
Console and the one the start pipeline sends.

## Credential 2 — `session-signing-key` (kind `jwt_signing_key`)

Same credential as in `cookie-jwt-auth`. For a browser flow set these on it:

```json
{
  "algorithm": "HS256",
  "secret": "…",
  "auth_redirect": "/wh/OWNER/PROJECT/auth/google/start"
}
```

`--auth-type jwt` reads the session cookie named `zebflow_session` by default.
`auth_redirect` is where an unauthenticated *browser* is sent (an API call
gets 401 JSON instead).

## The table

```sql
CREATE TABLE IF NOT EXISTS members (
  email       TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  roles       TEXT NOT NULL DEFAULT '["member"]',
  accepted_at INTEGER NOT NULL
);
```

Membership is decided by a human before the visitor ever signs in. Google
proves *who* the visitor is; this table decides *whether they may enter*.

---

## Pipeline 1 — GET /auth/google/start

```zf
register auth/google-start --
| trigger.webhook --path /auth/google/start --method GET
| crypto --op random_hex --length 16
| kv.set --key "oauth:state:{{ input.result }}" --ttl 600
| script -- "const q = { client_id: 'YOUR_CLIENT_ID.apps.googleusercontent.com', redirect_uri: 'https://your.site/wh/OWNER/PROJECT/auth/google/callback', response_type: 'code', scope: 'openid email profile', state: input.result, access_type: 'online', prompt: 'select_account' }; const qs = Object.keys(q).map(function (k) { return encodeURIComponent(k) + '=' + encodeURIComponent(q[k]); }).join('&'); return { auth_url: 'https://accounts.google.com/o/oauth2/v2/auth?' + qs };"
| web.response --location "{{ input.auth_url }}"
```

- `crypto --op random_hex` yields `{ result }`; that value is the OAuth `state`.
- `kv.set` remembers the state for ten minutes. A callback whose state is not in
  KV was not started by this server — that is the CSRF check.
- The client id is public by design; it may live in the pipeline. The client
  secret may not.
- `access_type: 'online'` — login does not need a refresh token, so do not ask
  for one.

## Pipeline 2 — GET /auth/google/callback

Graph form, because "is this e-mail a member" is a real branch.

```zf
register auth/google-callback --
[in] trigger.webhook --path /auth/google/callback --method GET
[state] kv.get --key "oauth:state:{{ input.query.state }}" --out-key state_record
[check] script -- "if (!input.query || !input.query.code) throw new Error('no authorization code in the callback'); if (input.state_record === null || input.state_record === undefined) throw new Error('unknown or expired state: this callback did not come from a sign-in this server started'); return { code: input.query.code, state: input.query.state };"
[burn] kv.del --key "oauth:state:{{ input.state }}"
[exchange] http.request --credential google-token-exchange --bind CODE=input.code
[identity] script -- "const b = (input.response && input.response.body) || {}; const idt = b.id_token; if (!idt) throw new Error('google returned no id_token'); const seg = idt.split('.')[1]; const claims = JSON.parse(atob(seg.replace(/-/g, '+').replace(/_/g, '/'))); if (!claims.email) throw new Error('id_token carried no email'); return { email: String(claims.email).toLowerCase(), name: claims.name || '' };"
[member] sqlite.query --query "SELECT email, name, roles FROM members WHERE email = ?1" --params "{{ [input.email] }}"
[known] logic.if --expr "input.rows && input.rows.length > 0"
[claim] script -- "const m = input.rows[0]; return { sub: m.email, name: m.name, roles: JSON.parse(m.roles) };"
[token] auth.token.create --credential session-signing-key --claim "sub={{ input.sub }}" --claim "name={{ input.name }}:public" --claim "roles={{ input.roles }}" --expires-in 86400
[welcome] web.response --location /wh/OWNER/PROJECT/me --set-cookie "name=session,value={{ input.access_token }},http-only,same-site=Lax,max-age=86400,path=/"
[stranger] web.response --status 403 --message "This Google account is not a member yet."
[in] -> [state]
[state] -> [check]
[check] -> [burn]
[burn] -> [exchange]
[exchange] -> [identity]
[identity] -> [member]
[member] -> [known]
[known]:true -> [claim]
[known]:false -> [stranger]
[claim] -> [token]
[token] -> [welcome]
```

Node by node:

- `[state]` — `kv.get` merges `{ state_record }` into the payload; `input.query`
  is still there for the next node.
- `[check]` — a script that throws stops the pipeline with that message in the
  trace. It is the right tool for "this request is malformed, refuse it".
- `[burn]` — a state is single-use. Delete it before the exchange so a replayed
  callback fails at `[check]`.
- `[exchange]` — the only node that touches the secret, and it never sees it:
  the credential owns the whole request, the node contributes `CODE`. Output is
  `{ request: { secured: true, … }, response: { status, ok, body } }`; `body` is
  already parsed JSON.
- `[identity]` — decodes the `id_token` payload. Its signature is **not**
  verified here, and that is correct for this flow: the token arrived over TLS
  directly from `oauth2.googleapis.com` in reply to a request this server made
  with its own client secret. Verify signatures on tokens that arrive from the
  *browser*; this one did not.
- `[member]` → `[known]` — a `logic.if` with `true`/`false` pins. The 403 is a
  `web.response` on the `false` pin, not a status returned from a script.
- `[token]` → `[welcome]` — from here on it is `cookie-jwt-auth`: `sub` is the
  member's e-mail, `roles` is the array `--auth-required-role` checks, and only
  `name` is `:public`.

## Protected routes

```zf
register auth/me --
| trigger.webhook --path /me --method GET --auth-type jwt --auth-credential session-signing-key
| sqlite.query --query "SELECT email, name, accepted_at FROM members WHERE email = ?1" --params "{{ [input.auth.sub] }}"
| web.response --template pages/me.tsx
```

```zf
register auth/admin --
| trigger.webhook --path /auth/admin --method GET --auth-type jwt --auth-credential session-signing-key --auth-required-role admin
| sqlite.query --query "SELECT email, name, roles, accepted_at FROM members ORDER BY accepted_at DESC"
| web.response --template pages/auth-admin.tsx
```

A member without `admin` in `roles` gets 403 (or `auth_forbidden_redirect`,
when the credential sets one) before the first node runs.

Neither route mentions Google. Swap the start/callback pair for a GitHub or a
password form and every protected route stays as it is.

---

## Nodes Used

- `crypto --op random_hex --length <bytes>` — output `{ result }`
- `kv.set --key <k> --ttl <secs>` / `kv.get --key <k> --out-key <k>` / `kv.del --key <k>` — state store; `kv.get` merges, `kv.del` passes the payload through
- `http.request --credential <secure_request id> --bind NAME=<expr>` — the credential owns URL, method, headers and body; one `--bind` per declared variable; output `{ request, response }`
- `sqlite.query --params "{{ [expr] }}"` — `?1` placeholders
- `logic.if --expr <js>` — `true` / `false` pins
- `auth.token.create --credential <jwt_signing_key id> --claim "k={{ v }}" [--claim "k={{ v }}:public"]` — output `{ access_token }`; quote each claim, an unquoted `{{ }}` is cut at its first space
- `web.response --location <url> --set-cookie <spec>` / `--status 403 --message <text>`
