# Sign in to a published MCP server (OAuth)

## What this builds

A published MCP server that ChatGPT and claude.ai connectors can add, where
each person signs in with **the app's own account**. The connector finds the
server's sign-in by itself, sends the person to the app's login page, and gets
back the app's tokens. No Zebflow account, session or cookie takes part: the
person is a user of the app, and the token is one the app signed.

```
connector ─▶ /_mcp/library            401: where to sign in
          ─▶ /.well-known/…           the server's OAuth documents
          ─▶ /_mcp/library/_oauth/authorize ─▶ /auth/login?oauth=…&client=…&redirect_host=…
person    ─▶ the app's login page     the app checks the password
app       ─▶ auth.oauth.approve       ─▶ back to the connector with a code
connector ─▶ /_mcp/library/_oauth/token ─▶ tools/list, tools/call
```

---

## What you need

1. A `jwt_signing_key` credential — here `library-jwt`. It signs the app's own
   session tokens (`auth.token.create`) and the server's access tokens.
2. The `mcp` surface switched on (Settings → Addressing). The connector's URL
   is `https://<project host>/_mcp/library`.
3. The tool pipelines, the login page and the login handler below.

---

## Pipelines

### The tool — `--auth oauth`

```
| trigger.mcp --route /library --name search --description "Find a book in the reading list by words." --parameter q:string! "The words to look for." --auth oauth --credential library-jwt --login /auth/login --role reader
| sekejap.query.run --param "1={{ input.mcp.arguments.q }}" -- "SELECT title, author FROM books WHERE title ILIKE '%' || $1 || '%'"
| web.response.send --body "{{ input.query.rows }}"
```

Every `trigger.mcp` on `/library` declares the same `--auth oauth
--credential library-jwt --login /auth/login`. `--role reader` admits only
tokens whose `roles` claim holds `reader`.

### GET /auth/login — the login page

The server sends the person here with three query values. Show the last two
— they say which app is asking and where the sign-in goes — and carry all
three in the form:

```
| trigger.webhook --route /auth/login --method GET
| web.response.send --template pages/login.tsx
```

`pages/login.tsx` reads `input.webhook.query.oauth`, `.client` and
`.redirect_host`, shows "Sign in to let **{client}** use your reading list
(it will go back to **{redirect_host}**)", and posts the identifier, password
and the three values as hidden fields to `POST /auth/login`.

### POST /auth/login — check the person, then approve

```
[t] trigger.webhook --route /auth/login --method POST
[user] sekejap.query.run --param "1={{ input.webhook.body.identifier }}" -- "SELECT id, password_hash, roles FROM members WHERE identifier = $1"
[pw] crypto.password.verify --from "{{ input.webhook.body.password }}" --hash "{{ input.query.rows[0]?.password_hash ?? '' }}"
[sign] auth.token.create --credential library-jwt --claim "sub={{ $nodes.user.query.rows[0].id }}" --claim "roles={{ $nodes.user.query.rows[0].roles }}"
[approve] auth.oauth.approve --ticket "{{ $trigger.body.oauth }}" --token "{{ input.token.access_token }}" --client "{{ $trigger.body.client }}" --redirect-host "{{ $trigger.body.redirect_host }}"
[wrong] web.response.send --status 401 --template pages/login.tsx
[expired] web.response.send --status 400 --template pages/link-expired.tsx
[t] -> [user]
[user] -> [pw]
[pw]:true -> [sign]
[pw]:false -> [wrong]
[pw]:error -> [wrong]
[sign] -> [approve]
[approve]:error -> [expired]
```

`auth.oauth.approve` answers the browser itself: a 303 back to the connector
with a two-minute code. On `:error` — a link older than ten minutes, used
twice, or a `client` / `redirect_host` the page did not show — it delivers
`oauth: { ok: false, error: { code, message } }`, and the app shows its own
"this link has expired" page.

---

## What the server does for you

| Address (connect URL `R` = `https://host/_mcp/library`) | |
|---|---|
| `/.well-known/oauth-protected-resource/_mcp/library` | RFC 9728: `resource` = `R`, the authorization server = `R` |
| `/.well-known/oauth-authorization-server/_mcp/library` | RFC 8414: issuer `R`, S256 PKCE, public clients |
| `R/_oauth/register` | RFC 7591 registration (a client may also use a metadata-document URL as its id) |
| `R/_oauth/authorize` | checks the client and redirect, then sends the person to `--login` |
| `R/_oauth/token` | code (with PKCE) or refresh token → the access token |

- Access tokens are JWTs signed with `--credential`, an hour long, carrying the
  app token's claims (`sub`, `roles`, …) and `aud` = `R`: they open this route
  and nothing else — not another route, not a `--auth jwt` webhook.
- Refresh tokens rotate; one used twice ends the whole sign-in.
- Codes last two minutes and are used once; tickets ten minutes.
- A role change takes effect at the next sign-in: a token carries the roles
  it was approved with.

---

## Nodes Used

- `trigger.mcp --auth oauth --credential <jwt key> --login <path>` — the published tool
- `auth.token.create` — the app's own token for the person
- `auth.oauth.approve --ticket --token --client --redirect-host` — completes the sign-in and answers the redirect
- `crypto.password.verify` — the app's own password check
