# Blog with Admin

## What this builds

A public blog with a paginated listing and post detail pages, plus a
JWT-protected admin panel for creating, editing, and deleting posts. Posts and
the one admin account live in Sekejap.

---

## Pipelines

1. `GET /blog` → list published posts → render listing page
2. `GET /blog/:slug` → fetch post by slug → render detail page
3. `GET /admin/posts` → JWT-gated → list all posts → render admin panel
4. `POST /api/posts` → JWT-gated → create or update post → return JSON
5. `DELETE /api/posts/:slug` → JWT-gated → delete post → return JSON
6. `GET /auth/login` → render the login form
7. `POST /auth/login` → verify credentials → issue JWT → redirect to the admin panel

---

## Tables

The post's slug is its key — one lookup, no separate id column.

```sql
CREATE TABLE posts (_key TEXT PRIMARY KEY, title TEXT, body TEXT, published BOOLEAN, created_at INTEGER, updated_at INTEGER)
CREATE TABLE users (_key TEXT PRIMARY KEY, password_hash TEXT, roles JSON)
```

## The credential

A `jwt_signing_key` credential, here `blog-jwt`, signs the session and
checks it on every `--auth jwt` route. Its `secret` is a long random
string entered in the credential form and never written in a file; the
other fields:

```json
{
  "algorithm": "HS256",
  "auth_roles": ["admin"],
  "auth_redirect": "/auth/login",
  "cookie_name": "session"
}
```

`--auth jwt` reads the token from `Authorization: Bearer`, else from the
cookie `cookie_name` names. The login below sets a cookie called `session`;
without `"cookie_name": "session"` the routes look for `zebflow_session`
instead and answer every signed-in request as signed out.

Seed one admin user. `javascript.script.run` adds its return value as `script`, keeping
the rest of the payload; `crypto.password.hash` then adds `password: { hash,
algorithm }` alongside it, so the next node reads `input.script.username` and
`input.script.roles`:

```zf
run
| javascript.script.run -- "return { username: 'admin', password: 'changeme', roles: ['admin'] }"
| crypto.password.hash --from "{{ input.script.password }}"
| sekejap.query.run --write --param "1={{ input.script.username }}" --param "2={{ input.password.hash }}" --param "3={{ input.script.roles }}" -- "INSERT INTO users (_key, password_hash, roles) VALUES ($1, $2, $3)"
```

---

## DSL

### blog-list — public post listing

```
| trigger.webhook --route /blog --method GET
| sekejap.query.run -- "SELECT _key, title, body, created_at FROM posts WHERE published = true ORDER BY created_at DESC LIMIT 20"
| javascript.script.run -- "return { posts: input.query.rows }"
| web.response.send --template pages/blog-home.tsx
```

### blog-detail — single post

```zf
register blog/detail --
[a] trigger.webhook --route /blog/:slug --method GET
[b] sekejap.query.run --param "1={{ input.webhook.params.slug }}" -- "SELECT * FROM posts WHERE _key = $1 AND published = true"
[c] logic.if --when "input.query.rows.length > 0"
[d] javascript.script.run -- "return { post: input.query.rows[0] };"
[e] web.response.send --template pages/blog-detail.tsx
[f] web.response.send --status 302 --header "Location=/blog"

[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[d] -> [e]
[c]:false -> [f]
```

### admin-list — protected admin panel

Authenticate at the door: the trigger itself checks the JWT, so the rest of
the pipeline only ever runs for someone already verified.

```
| trigger.webhook --route /admin/posts --method GET --auth jwt --credential blog-jwt --role admin
| sekejap.query.run -- "SELECT _key, title, published, created_at FROM posts ORDER BY created_at DESC"
| javascript.script.run -- "return { posts: input.query.rows }"
| web.response.send --template pages/admin-posts.tsx
```

### api-post-upsert — create or update post

One statement creates the post or updates it: `ON CONFLICT (_key) DO UPDATE`
keeps the row's `created_at` and changes only what an edit changes.

```zf
register blog/api-post-upsert --
[trig] trigger.webhook --route /api/posts --method POST --auth jwt --credential blog-jwt --role admin
[has_title] logic.if --when "!!(input.webhook.body && input.webhook.body.title)"
[bad] web.response.send --status 400 --body "{{ { ok: false, error: 'title required' } }}"
[draft] javascript.script.run -- "const slug = $trigger.body.slug || String($trigger.body.title).toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { slug, title: $trigger.body.title, body: $trigger.body.body || '', published: !!$trigger.body.published };"
[save] sekejap.query.run --write --param "1={{ $nodes.draft.script.slug }}" --param "2={{ $nodes.draft.script.title }}" --param "3={{ $nodes.draft.script.body }}" --param "4={{ $nodes.draft.script.published }}" --param "5={{ Date.now() }}" -- "INSERT INTO posts (_key, title, body, published, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $5) ON CONFLICT (_key) DO UPDATE SET title = EXCLUDED.title, body = EXCLUDED.body, published = EXCLUDED.published, updated_at = EXCLUDED.updated_at"
[ok] web.response.send --body "{{ { ok: true, slug: $nodes.draft.script.slug } }}"

[trig] -> [has_title]
[has_title]:false -> [bad]
[has_title]:true -> [draft]
[draft] -> [save]
[save] -> [ok]
```

### api-post-delete — delete post

```
| trigger.webhook --route /api/posts/:slug --method DELETE --auth jwt --credential blog-jwt --role admin
| sekejap.query.run --write --param "1={{ input.webhook.params.slug }}" -- "DELETE FROM posts WHERE _key = $1"
| web.response.send --body "{{ { ok: true, deleted: input.query.rows_affected > 0 } }}"
```

A pipeline that ends without `web.response.send` answers `204 No Content`,
so the route names its answer.

### auth-login-page: the login form

```
| trigger.webhook --route /auth/login --method GET
| web.response.send --template pages/login.tsx
```

`pages/login.tsx` is a form that posts `username` and `password` to
`/auth/login`; the credential's `auth_redirect` sends a signed-out visitor of
`/admin/posts` here.

### auth-login — issue JWT

```zf
register blog/auth-login --
[trig] trigger.webhook --route /auth/login --method POST
[lookup] sekejap.query.run --param "1={{ input.webhook.body.username }}" -- "SELECT _key, password_hash, roles FROM users WHERE _key = $1"
[found] logic.if --when "input.query.rows.length > 0"
[verify] crypto.password.verify --from "{{ $trigger.body.password }}" --hash "{{ input.query.rows[0]?.password_hash }}"
[token] auth.token.create --credential blog-jwt --claim "sub={{ input.query.rows[0]._key }}" --claim "roles:public={{ input.query.rows[0].roles }}" --ttl 1d
[welcome] web.response.send --status 302 --header "Location=/admin/posts" --header "Set-Cookie=session={{ input.token.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
[denied] web.response.send --status 401 --body "invalid credentials"

[trig] -> [lookup]
[lookup] -> [found]
[found]:true -> [verify]
[found]:false -> [denied]
[verify]:true -> [token]
[verify]:false -> [denied]
[token] -> [welcome]
```

Never compare a password with `===`, and never read one from an environment
variable. `crypto.password.verify` answers on `true`/`false` pins and
adds `password: { valid }`, keeping the rest of the payload — including
`input.query.rows` from `[lookup]` — so the comparison is both the branch and
constant-time.

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints (GET, POST, DELETE); `--auth jwt` gates admin routes
- `sekejap.query.run` — SQL against Sekejap; no `--table`/`--op`, just `SELECT`/`INSERT`/`UPDATE`/`DELETE` with `--param`, and `--write` for a write
- `logic.if` — branch on "does this slug/user already exist"
- `javascript.script.run` — slugify, validate, shape rows; return value is added as `script`, the rest of the payload is kept
- `crypto.password.hash` to seed the password, `crypto.password.verify` to check it
- `auth.token.create --ttl <duration>` / `web.response.send --header "Set-Cookie=…"` — issue the session; read `input.token.access_token`
- `web.response.send` — TSX templates for public and admin pages

---

## Templates Needed

- `pages/blog-home.tsx` — post listing
- `pages/blog-detail.tsx` — single post display
- `pages/admin-posts.tsx` — admin CRUD interface
- `pages/login.tsx`: the login form

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
