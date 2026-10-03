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
6. `POST /auth/login` → verify credentials → issue JWT → redirect

---

## Tables

The post's slug is its key — one lookup, no separate id column.

```sql
CREATE TABLE posts (_key TEXT PRIMARY KEY, title TEXT, body TEXT, published BOOLEAN, created_at INTEGER, updated_at INTEGER)
CREATE TABLE users (_key TEXT PRIMARY KEY, password_hash TEXT, roles JSON)
```

Seed one admin user. `crypto.password.hash` adds `password: { hash, algorithm }`
to the payload and keeps the rest, so the next node still sees `username` and `roles`:

```zf
run
| script.result.run -- "return { username: 'admin', password: 'changeme', roles: ['admin'] }"
| crypto.password.hash --from "{{ input.password }}"
| sekejap.query.run --write --param "1={{ input.username }}" --param "2={{ input.password.hash }}" --param "3={{ input.roles }}" -- "INSERT INTO users (_key, password_hash, roles) VALUES ($1, $2, $3)"
```

---

## DSL

### blog-list — public post listing

```
| trigger.webhook --path /blog --method GET
| sekejap.query.run -- "SELECT * FROM posts WHERE published = true ORDER BY created_at DESC LIMIT 20"
| script.result.run -- "return { posts: input.query.rows }"
| web.response.send --template pages/blog-home.tsx
```

### blog-detail — single post

```zf
register blog/detail --
[a] trigger.webhook --path /blog/:slug --method GET
[b] sekejap.query.run --param "1={{ input.params.slug }}" -- "SELECT * FROM posts WHERE _key = $1 AND published = true"
[c] logic.if --expr "input.query.rows.length > 0"
[d] script.result.run -- "return { post: input.query.rows[0] };"
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
| trigger.webhook --path /admin/posts --method GET --auth-type jwt --auth-credential blog-jwt --auth-required-role admin
| sekejap.query.run -- "SELECT * FROM posts ORDER BY created_at DESC"
| script.result.run -- "return { posts: input.query.rows }"
| web.response.send --template pages/admin-posts.tsx
```

### api-post-upsert — create or update post

Sekejap has no `UPSERT`/`ON CONFLICT`. Look the slug up first, then branch:

```zf
register blog/api-post-upsert --
[trig] trigger.webhook --path /api/posts --method POST --auth-type jwt --auth-credential blog-jwt --auth-required-role admin
[has_title] logic.if --expr "!!(input.body && input.body.title)"
[bad] web.response.send --status 400 --body "{{ { ok: false, error: 'title required' } }}"
[draft] script.result.run -- "const slug = input.body.slug || String(input.body.title).toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { slug, title: input.body.title, body: input.body.body || '', published: !!input.body.published };"
[find] sekejap.query.run --param "1={{ $nodes.draft.slug }}" -- "SELECT _key FROM posts WHERE _key = $1"
[exists] logic.if --expr "input.query.rows.length > 0"
[update] sekejap.query.run --write --param "1={{ $nodes.draft.title }}" --param "2={{ $nodes.draft.body }}" --param "3={{ $nodes.draft.published }}" --param "4={{ Date.now() }}" --param "5={{ $nodes.draft.slug }}" -- "UPDATE posts SET title = $1, body = $2, published = $3, updated_at = $4 WHERE _key = $5"
[insert] sekejap.query.run --write --param "1={{ $nodes.draft.slug }}" --param "2={{ $nodes.draft.title }}" --param "3={{ $nodes.draft.body }}" --param "4={{ $nodes.draft.published }}" --param "5={{ Date.now() }}" --param "6={{ Date.now() }}" -- "INSERT INTO posts (_key, title, body, published, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
[ok] web.response.send --body "{{ { ok: true, slug: $nodes.draft.slug } }}"

[trig] -> [has_title]
[has_title]:false -> [bad]
[has_title]:true -> [draft]
[draft] -> [find]
[find] -> [exists]
[exists]:true -> [update]
[exists]:false -> [insert]
[update] -> [ok]
[insert] -> [ok]
```

`$nodes.draft.slug` reaches back to the `[draft]` script's output by its
graph id — it still resolves after `[find]`'s `sekejap.query.run` has replaced
`input` with `{ query: { columns, rows, … } }`.

### api-post-delete — delete post

```
| trigger.webhook --path /api/posts/:slug --method DELETE --auth-type jwt --auth-credential blog-jwt --auth-required-role admin
| sekejap.query.run --write --param "1={{ input.params.slug }}" -- "DELETE FROM posts WHERE _key = $1"
| script.result.run -- "return { ok: true }"
```

### auth-login — issue JWT

```zf
register blog/auth-login --
[trig] trigger.webhook --path /auth/login --method POST
[lookup] sekejap.query.run --param "1={{ input.body.username }}" -- "SELECT _key, password_hash, roles FROM users WHERE _key = $1"
[found] logic.if --expr "input.query.rows.length > 0"
[verify] crypto.password.verify --from "{{ input.body.password }}" --hash "{{ input.query.rows[0]?.password_hash }}"
[token] auth.token.create --credential blog-jwt --claim "sub={{ input.query.rows[0]._key }}" --claim "roles:public={{ input.query.rows[0].roles }}" --expires-in 86400
[welcome] web.response.send --status 302 --header "Location=/admin" --header "Set-Cookie=session={{ input.access_token }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"
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

- `trigger.webhook` — HTTP endpoints (GET, POST, DELETE); `--auth-type jwt` gates admin routes
- `sekejap.query.run` — SQL against Sekejap; no `--table`/`--op`, just `SELECT`/`INSERT`/`UPDATE`/`DELETE` with `--param`, and `--write` for a write
- `logic.if` — branch on "does this slug/user already exist"
- `script` — slugify, validate, shape rows
- `crypto.password.hash` to seed the password, `crypto.password.verify` to check it
- `auth.token.create` / `web.response.send --header "Set-Cookie=…"` — issue the session
- `web.response.send` — TSX templates for public and admin pages

---

## Templates Needed

- `pages/blog-home.tsx` — post listing
- `pages/blog-detail.tsx` — single post display
- `pages/admin-posts.tsx` — admin CRUD interface

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response.send` answer —
> `--status`, `--header` (`Location`, `Set-Cookie`), `--body`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
