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

Seed one admin user. `crypto --op argon2_hash` replaces the whole payload with
`{ result }`, so the hash is the next node's entire input:

```zf
run
| script -- "return { password: 'changeme' }"
| crypto --op argon2_hash
| script -- "return { username: 'admin', password_hash: input.result, roles: ['admin'] }"
| sekejap.query --read-only false --params "{{ [input.username, input.password_hash, input.roles] }}" -- "INSERT INTO users (_key, password_hash, roles) VALUES ($1, $2, $3)"
```

---

## DSL

### blog-list — public post listing

```
| trigger.webhook --path /blog --method GET
| sekejap.query -- "SELECT * FROM posts WHERE published = true ORDER BY created_at DESC LIMIT 20"
| script -- "return { posts: input.rows }"
| web.response --template pages/blog-home.tsx
```

### blog-detail — single post

```zf
register blog/detail --
[a] trigger.webhook --path /blog/:slug --method GET
[b] sekejap.query --params "{{ [input.params.slug] }}" -- "SELECT * FROM posts WHERE _key = $1 AND published = true"
[c] logic.if --expr "input.rows.length > 0"
[d] script -- "return { post: input.rows[0] };"
[e] web.response --template pages/blog-detail.tsx
[f] web.response --location /blog

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
| sekejap.query -- "SELECT * FROM posts ORDER BY created_at DESC"
| script -- "return { posts: input.rows }"
| web.response --template pages/admin-posts.tsx
```

### api-post-upsert — create or update post

Sekejap has no `UPSERT`/`ON CONFLICT`. Look the slug up first, then branch:

```zf
register blog/api-post-upsert --
[trig] trigger.webhook --path /api/posts --method POST --auth-type jwt --auth-credential blog-jwt --auth-required-role admin
[has_title] logic.if --expr "!!(input.body && input.body.title)"
[bad] web.response --status 400 --body "{{ { ok: false, error: 'title required' } }}"
[draft] script -- "const slug = input.body.slug || String(input.body.title).toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { slug, title: input.body.title, body: input.body.body || '', published: !!input.body.published };"
[find] sekejap.query --params "{{ [$nodes.draft.slug] }}" -- "SELECT _key FROM posts WHERE _key = $1"
[exists] logic.if --expr "input.rows.length > 0"
[update] sekejap.query --read-only false --params "{{ [$nodes.draft.title, $nodes.draft.body, $nodes.draft.published, Date.now(), $nodes.draft.slug] }}" -- "UPDATE posts SET title = $1, body = $2, published = $3, updated_at = $4 WHERE _key = $5"
[insert] sekejap.query --read-only false --params "{{ [$nodes.draft.slug, $nodes.draft.title, $nodes.draft.body, $nodes.draft.published, Date.now(), Date.now()] }}" -- "INSERT INTO posts (_key, title, body, published, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
[ok] web.response --body "{{ { ok: true, slug: $nodes.draft.slug } }}"

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
graph id — it still resolves after `[find]`'s `sekejap.query` has replaced
`input` with `{ columns, rows, … }`.

### api-post-delete — delete post

```
| trigger.webhook --path /api/posts/:slug --method DELETE --auth-type jwt --auth-credential blog-jwt --auth-required-role admin
| sekejap.query --read-only false --params "{{ [input.params.slug] }}" -- "DELETE FROM posts WHERE _key = $1"
| script -- "return { ok: true }"
```

### auth-login — issue JWT

```zf
register blog/auth-login --
[trig] trigger.webhook --path /auth/login --method POST
[lookup] sekejap.query --params "{{ [input.body.username] }}" -- "SELECT _key, password_hash, roles FROM users WHERE _key = $1"
[found] logic.if --expr "input.rows.length > 0"
[verify] crypto --op argon2_verify --input "{{ input.body.password }}" --hash "{{ input.rows[0].password_hash }}"
[token] auth.token.create --credential blog-jwt --claim "sub={{ input.rows[0]._key }}" --claim "roles={{ input.rows[0].roles }}:public" --expires-in 86400
[welcome] web.response --location /admin --set-cookie "name=session,value={{ input.access_token }},http-only,max-age=86400,same-site=Lax,path=/"
[denied] web.response --status 401 --message "invalid credentials"

[trig] -> [lookup]
[lookup] -> [found]
[found]:true -> [verify]
[found]:false -> [denied]
[verify]:true -> [token]
[verify]:false -> [denied]
[token] -> [welcome]
```

Never compare a password with `===`, and never read one from an environment
variable. `crypto --op argon2_verify` answers on `true`/`false` pins and
forwards the payload — including `input.rows` from `[lookup]` — unchanged, so
the comparison is both the branch and constant-time.

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints (GET, POST, DELETE); `--auth-type jwt` gates admin routes
- `sekejap.query` — SQL against Sekejap; no `--table`/`--op`, just `SELECT`/`INSERT`/`UPDATE`/`DELETE` with `--params`
- `logic.if` — branch on "does this slug/user already exist"
- `script` — slugify, validate, shape rows
- `crypto` — `argon2_hash` to seed the password, `argon2_verify` to check it
- `auth.token.create` / `web.response --set-cookie` — issue the session
- `web.response` — TSX templates for public and admin pages

---

## Templates Needed

- `pages/blog-home.tsx` — post listing
- `pages/blog-detail.tsx` — single post display
- `pages/admin-posts.tsx` — admin CRUD interface

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
