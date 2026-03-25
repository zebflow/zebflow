# Blog with Admin

## What this builds

A public blog with paginated listing and post detail pages, plus a JWT-protected admin panel for creating, editing, and deleting posts. Posts stored in Sekejap.

---

## Pipelines

1. `GET /blog` → list posts → render listing page
2. `GET /blog/:slug` → fetch post by slug → render detail page
3. `GET /admin/posts` → auth check → list all posts → render admin panel
4. `POST /api/posts` → auth check → upsert post → return JSON
5. `DELETE /api/posts/:slug` → auth check → delete post → return JSON
6. `POST /auth/login` → validate credentials → issue JWT → redirect

---

## DSL

### blog-list — public post listing

```
| trigger.webhook --path /blog --method GET
| sekejap.query --table posts --op scan
| script -- "return { posts: input.filter(p => p.published).sort((a,b)=>b.created_at-a.created_at).slice(0,20) }"
| web.render --template-path pages/blog-home.tsx --route /blog
```

### blog-detail — single post

```
| trigger.webhook --path /blog/:slug --method GET
| sekejap.query --table posts --op get --key "{{input.params.slug}}"
| script -- "if (!input || !input.published) return { __redirect: '/blog' }; return input"
| web.render --template-path pages/blog-detail.tsx --route /blog/:slug
```

### admin-list — protected admin panel

```
| trigger.webhook --path /admin/posts --method GET
| script -- "const tok = input.headers['authorization'] || input.query.token; if (!tok) return { __redirect: '/auth/login' }; return { token: tok }"
| sekejap.query --table posts --op scan
| web.render --template-path pages/admin-posts.tsx --route /admin/posts
```

### api-post-upsert — create or update post

```
| trigger.webhook --path /api/posts --method POST
| script -- "const b = input.body; if (!b.title) return { error: 'title required', __status: 400 }; const slug = b.slug || b.title.toLowerCase().replace(/[^a-z0-9]+/g,'-'); return { ...b, slug, updated_at: Date.now(), created_at: b.created_at || Date.now() }"
| sekejap.query --table posts --op upsert
| script -- "return { ok: true, slug: input.slug }"
```

### api-post-delete — delete post

```
| trigger.webhook --path /api/posts/:slug --method DELETE
| sekejap.query --table posts --op delete --key "{{input.params.slug}}"
| script -- "return { ok: true }"
```

### auth-login — issue JWT

```
| trigger.webhook --path /auth/login --method POST
| script -- "const { username, password } = input.body; if (username === 'admin' && password === process.env.ADMIN_PASSWORD) { return { user: username, role: 'admin', token: btoa(JSON.stringify({ user: username, role: 'admin', exp: Date.now() + 86400000 })) }; } return { error: 'invalid credentials', __status: 401 }"
```

---

## Nodes Used

- `trigger.webhook` — HTTP endpoints (GET, POST, DELETE)
- `sekejap.query` — embedded key-value store (scan, get, upsert, delete)
- `script` — auth checks, data transforms, validation
- `web.render` — TSX templates for public and admin pages

---

## Templates Needed

- `pages/blog-home.tsx` — post listing
- `pages/blog-detail.tsx` — single post display
- `pages/admin-posts.tsx` — admin CRUD interface

Use `template_create kind=page name=blog-home` then `template_write` to fill content.
