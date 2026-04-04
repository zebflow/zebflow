# Full Project Workflow — Concept to Live Website

A thought experiment: an LLM agent connects via MCP and builds a complete website from scratch.
Scenario: **a personal blog with a public frontend and a private admin panel.**

This document shows the exact sequence of thinking and tool calls at every phase.

---

## Phase 0: Connect

Every session starts identically:

```
start_here
docs_agent_read  name=AGENTS.md
docs_agent_read  name=MEMORY.md
pipeline_list
template_list
connection_list
```

**What the agent learns on a fresh project:**
- AGENTS.md: "Blog for @acme. Tone is casual. DB is PostgreSQL (credential: `main-db`)."
- MEMORY.md: "(empty — first session)"
- pipeline_list: "(no pipelines)"
- template_list: "(no templates)"
- connection_list: "main-db (postgres)"

**Agent decision:** Fresh project. Read the domain skills before anything else.

```
help  topic="pipeline"
help  topic="web"
help  topic="platform/operations"
```

---

## Phase 1: Concept

The agent and owner clarify what "blog" means before touching any code.

**Agent writes a concept doc first:**

```
docs_project_write
  path=REQUIREMENTS.md
  content="""
  # Blog Concept

  ## What we're building
  A personal blog with:
  - Public frontend: post list, post detail, about page
  - Private admin: create/edit/delete posts, draft vs published

  ## Users
  - Visitors: read posts (no auth)
  - Owner: write posts (HTTP Basic auth or session token)

  ## Tech decisions
  - Data: PostgreSQL via credential `main-db`
  - Auth: HTTP Basic on admin routes (handled in pipeline script node)
  - No JS framework needed for public pages (hydration: static)
  - Admin panel needs reactivity (hydration: reactive)
  """
```

**Why write the doc first?** It forces clarity before any code. It's also committed to git — the owner can review and correct it before the agent builds anything.

---

## Phase 2: Data Model

Agent designs the schema and writes it as a spec doc before creating anything.

```
docs_project_write
  path=schema.md
  content="""
  # Database Schema

  ## posts
  - id          SERIAL PRIMARY KEY
  - slug        TEXT UNIQUE NOT NULL
  - title       TEXT NOT NULL
  - body        TEXT NOT NULL          -- markdown
  - status      TEXT DEFAULT 'draft'  -- 'draft' | 'published'
  - created_at  TIMESTAMPTZ DEFAULT now()
  - updated_at  TIMESTAMPTZ DEFAULT now()

  ## No other tables needed for v1.
  """
```

Agent checks DB schema:

```
connection_describe  slug=main-db  scope=tables
```

Agent notes: the `posts` table must be created in PostgreSQL by the owner. Records this in MEMORY.md:

```
docs_agent_write
  name=MEMORY.md
  content="""
  # Session 1 — Blog Setup

  ## Status: Phase 2 (schema designed)

  ## Prerequisite (owner must do)
  CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT DEFAULT 'draft',
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
  );

  ## Next
  Build pipelines + templates (Phase 3+)
  """
```

---

## Phase 3: Architecture Plan

Before writing any pipeline or template, the agent designs the full architecture:

```
docs_project_write
  path=architecture.md
  content="""
  # Architecture

  ## Pipelines
  1. blog-list      GET /blog           → query published posts → render pages/blog-list
  2. blog-post      GET /blog/:slug     → query single post    → render pages/blog-post
  3. blog-about     GET /about          → static               → render pages/blog-about
  4. admin-posts    GET /admin/posts    → auth check → query all posts → render pages/admin-posts
  5. admin-post-get GET /admin/post/:slug → auth → query post → render pages/admin-editor
  6. admin-post-put PUT /admin/post/:slug → auth → validate → upsert post → redirect

  ## Templates
  pages/blog-list.tsx       — post cards, pagination
  pages/blog-post.tsx       — post detail, markdown render
  pages/blog-about.tsx      — static about page
  pages/admin-posts.tsx     — table of all posts, status badges, edit links
  pages/admin-editor.tsx    — form: title, slug, body (markdown), status toggle, save

  ## Auth strategy
  Script node on all admin routes:
    const auth = input.headers['authorization'] ?? ''
    const [user, pass] = atob(auth.replace('Basic ', '')).split(':')
    if (user !== 'admin' || pass !== credentials.admin_pass.secret) {
      return { __redirect: '/login', status: 401 }
    }
    return input  // pass through
  """
```

**Agent commits the docs before any code:**

```
git_command  subcommand=add      args="docs/"
git_command  subcommand=commit   message="docs: concept, schema, architecture"
```

---

## Phase 4: Build Pipelines

Agent builds pipelines one by one. Register first (draft), then scaffold template, then activate together.

### 4.1 Public blog list

```
pipeline_register
  file_rel_path=pipelines/pages/blog-list.zf.json
  body="""
  | trigger.webhook --path /blog --method GET
  | pg.query --credential main-db -- "
      SELECT id, slug, title, created_at
      FROM posts
      WHERE status = 'published'
      ORDER BY created_at DESC
      LIMIT 20
    "
  | web.response --template pages/blog-list.tsx
  """
```

### 4.2 Blog post detail (slug from query param)

```
pipeline_register
  file_rel_path=pipelines/pages/blog-post.zf.json
  body="""
  | trigger.webhook --path /blog/post --method GET
  | pg.query --credential main-db -- "
      SELECT id, slug, title, body, created_at
      FROM posts
      WHERE slug = '{{input.query.slug}}'
        AND status = 'published'
      LIMIT 1
    "
  | web.response --template pages/blog-post.tsx
  """
```

### 4.3 Admin posts list

```
pipeline_register
  file_rel_path=pipelines/admin/admin-posts.zf.json
  body="""
  | trigger.webhook --path /admin/posts --method GET
  | script -- "
      const auth = input.headers['authorization'] ?? ''
      try {
        const [user, pass] = atob(auth.replace('Basic ', '')).split(':')
        if (user !== 'admin') return { __status: 401, __body: 'Unauthorized' }
      } catch { return { __status: 401, __body: 'Unauthorized' } }
      return input
    "
  | pg.query --credential main-db -- "
      SELECT id, slug, title, status, created_at
      FROM posts
      ORDER BY created_at DESC
    "
  | web.response --template pages/admin-posts.tsx
  """
```

### 4.4 Admin post editor (GET)

```
pipeline_register
  file_rel_path=pipelines/admin/admin-post-get.zf.json
  body="""
  | trigger.webhook --path /admin/post --method GET
  | script -- "
      const auth = input.headers['authorization'] ?? ''
      try {
        const [user, pass] = atob(auth.replace('Basic ', '')).split(':')
        if (user !== 'admin') return { __status: 401, __body: 'Unauthorized' }
      } catch { return { __status: 401, __body: 'Unauthorized' } }
      return input
    "
  | pg.query --credential main-db -- "
      SELECT id, slug, title, body, status
      FROM posts
      WHERE slug = '{{input.query.slug}}'
      LIMIT 1
    "
  | web.response --template pages/admin-editor.tsx
  """
```

### 4.5 Admin post save (PUT)

```
pipeline_register
  file_rel_path=pipelines/admin/admin-post-put.zf.json
  body="""
  | trigger.webhook --path /admin/post --method PUT
  | script -- "
      const auth = input.headers['authorization'] ?? ''
      try {
        const [user, pass] = atob(auth.replace('Basic ', '')).split(':')
        if (user !== 'admin') return { __status: 401, __body: 'Unauthorized' }
      } catch { return { __status: 401, __body: 'Unauthorized' } }
      const { slug, title, body, status } = input
      if (!slug || !title || !body) return { __status: 400, __body: 'Missing fields' }
      return { slug, title, body, status: status || 'draft' }
    "
  | pg.query --credential main-db -- "
      INSERT INTO posts (slug, title, body, status)
      VALUES ('{{input.slug}}', '{{input.title}}', '{{input.body}}', '{{input.status}}')
      ON CONFLICT (slug) DO UPDATE
        SET title = EXCLUDED.title,
            body  = EXCLUDED.body,
            status = EXCLUDED.status,
            updated_at = now()
      RETURNING id, slug
    "
  | script -- "return { __redirect: '/admin/posts', __status: 302 }"
  """
```

**Verify all pipelines registered:**

```
pipeline_list
```

Output shows 5 pipelines, all `draft`.

---

## Phase 5: Build Templates

Agent creates each template. Pattern: `template_create` → inspect scaffold → `template_write` with real content.

### 5.1 Blog list page

```
template_create  kind=page  name=blog-list
template_get     rel_path=pages/blog-list.tsx
```

Agent sees the scaffold, then writes the real component:

```
template_write
  rel_path=pages/blog-list.tsx
  content="""
  import Button from "@/components/ui/button";
  import Badge from "@/components/ui/badge";

  export default function BlogList(input) {
    const posts = input.state?.rows ?? [];
    return (
      <div className="max-w-2xl mx-auto px-4 py-12">
        <h1 className="text-4xl font-bold text-slate-900 mb-2">Blog</h1>
        <p className="text-slate-500 mb-10">Thoughts on code and building things.</p>
        <div className="space-y-6">
          {posts.map(p => (
            <a key={p.id} href={`/blog/post?slug=${p.slug}`}
               className="block group p-6 rounded-xl border border-slate-200 hover:border-sky-300 transition-colors">
              <p className="text-xs text-slate-400 mb-1">{p.created_at?.slice(0, 10)}</p>
              <h2 className="text-xl font-semibold text-slate-800 group-hover:text-sky-600 transition-colors">
                {p.title}
              </h2>
            </a>
          ))}
        </div>
      </div>
    );
  }

  export const page = { title: "Blog", description: "All posts" };
  export const app  = { hydration: "static" };
  """
```

### 5.2 Post detail

```
template_create  kind=page  name=blog-post
template_write
  rel_path=pages/blog-post.tsx
  content="""
  import Markdown from "@/components/ui/markdown";

  export default function BlogPost(input) {
    const post = input.state?.rows?.[0];
    if (!post) return (
      <div className="max-w-2xl mx-auto px-4 py-12">
        <h1 className="text-2xl font-bold text-slate-800">Post not found</h1>
        <a href="/blog" className="text-sky-500 mt-4 inline-block">← Back</a>
      </div>
    );
    return (
      <article className="max-w-2xl mx-auto px-4 py-12">
        <a href="/blog" className="text-sky-500 text-sm mb-6 inline-block">← All posts</a>
        <h1 className="text-4xl font-bold text-slate-900 mb-2">{post.title}</h1>
        <p className="text-slate-400 text-sm mb-10">{post.created_at?.slice(0, 10)}</p>
        <Markdown content={post.body} className="prose prose-slate max-w-none" />
      </article>
    );
  }

  export const page = { title: "Post" };
  export const app  = { hydration: "static" };
  """
```

### 5.3 Admin posts table

```
template_create  kind=page  name=admin-posts
template_write
  rel_path=pages/admin-posts.tsx
  content="""
  import Badge from "@/components/ui/badge";
  import Button from "@/components/ui/button";

  export default function AdminPosts(input) {
    const posts = input.state?.rows ?? [];
    return (
      <div className="max-w-4xl mx-auto px-4 py-10">
        <div className="flex items-center justify-between mb-8">
          <h1 className="text-2xl font-bold text-slate-900">Posts</h1>
          <Button asChild><a href="/admin/post?slug=new">New post</a></Button>
        </div>
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-slate-400 border-b border-slate-200">
              <th className="pb-2 font-medium">Title</th>
              <th className="pb-2 font-medium">Status</th>
              <th className="pb-2 font-medium">Date</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {posts.map(p => (
              <tr key={p.id} className="border-b border-slate-100">
                <td className="py-3 text-slate-800">{p.title}</td>
                <td className="py-3">
                  <Badge variant={p.status === 'published' ? 'success' : 'default'}>
                    {p.status}
                  </Badge>
                </td>
                <td className="py-3 text-slate-400">{p.created_at?.slice(0, 10)}</td>
                <td className="py-3">
                  <a href={`/admin/post?slug=${p.slug}`}
                     className="text-sky-500 hover:underline text-xs">Edit</a>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  export const page = { title: "Admin — Posts" };
  export const app  = { hydration: "static" };
  """
```

### 5.4 Admin editor

```
template_create  kind=page  name=admin-editor
template_write
  rel_path=pages/admin-editor.tsx
  content="""
  import Button from "@/components/ui/button";
  import Input from "@/components/ui/input";
  import Field from "@/components/ui/field";
  import Label from "@/components/ui/label";

  export default function AdminEditor(input) {
    const post = input.state?.rows?.[0] ?? {};
    const [slug, setSlug]     = useState(post.slug    ?? '');
    const [title, setTitle]   = useState(post.title   ?? '');
    const [body, setBody]     = useState(post.body    ?? '');
    const [status, setStatus] = useState(post.status  ?? 'draft');
    const [saving, setSaving] = useState(false);

    async function save() {
      setSaving(true);
      await fetch('/admin/post', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ slug, title, body, status }),
      });
      window.location.href = '/admin/posts';
    }

    return (
      <div className="max-w-2xl mx-auto px-4 py-10">
        <div className="flex items-center justify-between mb-8">
          <h1 className="text-2xl font-bold text-slate-900">
            {post.slug ? 'Edit post' : 'New post'}
          </h1>
          <a href="/admin/posts" className="text-sm text-slate-400 hover:text-slate-600">Cancel</a>
        </div>
        <div className="space-y-5">
          <Field>
            <Label>Slug</Label>
            <Input value={slug} onInput={e => setSlug(e.target.value)}
                   placeholder="my-post-slug" disabled={!!post.slug} />
          </Field>
          <Field>
            <Label>Title</Label>
            <Input value={title} onInput={e => setTitle(e.target.value)}
                   placeholder="Post title" />
          </Field>
          <Field>
            <Label>Body (Markdown)</Label>
            <textarea
              className="w-full min-h-64 rounded-md border border-slate-200 px-3 py-2 text-sm
                         font-mono focus:outline-none focus:ring-2 focus:ring-sky-500"
              value={body}
              onInput={e => setBody(e.target.value)}
              placeholder="Write in markdown..."
            />
          </Field>
          <div className="flex items-center justify-between pt-2">
            <label className="flex items-center gap-2 text-sm text-slate-700">
              <input type="checkbox"
                checked={status === 'published'}
                onChange={e => setStatus(e.target.checked ? 'published' : 'draft')}
              />
              Published
            </label>
            <Button onClick={save} disabled={saving}>
              {saving ? 'Saving…' : 'Save post'}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  export const page = { title: "Admin — Editor" };
  export const app  = { hydration: "reactive" };
  """
```

---

## Phase 6: Activate + Verify

Agent activates all pipelines:

```
pipeline_activate  file_rel_path=pipelines/pages/blog-list.zf.json
pipeline_activate  file_rel_path=pipelines/pages/blog-post.zf.json
pipeline_activate  file_rel_path=pipelines/admin/admin-posts.zf.json
pipeline_activate  file_rel_path=pipelines/admin/admin-post-get.zf.json
pipeline_activate  file_rel_path=pipelines/admin/admin-post-put.zf.json
```

Verify everything is active:

```
pipeline_list
```

All 5 show `active`. Routes are now live:

| Route | Pipeline | Template |
|-------|----------|----------|
| `GET /blog` | blog-list | pages/blog-list.tsx |
| `GET /blog/post?slug=...` | blog-post | pages/blog-post.tsx |
| `GET /admin/posts` | admin-posts | pages/admin-posts.tsx |
| `GET /admin/post?slug=...` | admin-post-get | pages/admin-editor.tsx |
| `PUT /admin/post` | admin-post-put | — (redirects) |

**Agent test-executes a pipeline to verify node-level behaviour:**

```
pipeline_execute
  file_rel_path=pipelines/pages/blog-list.zf.json
  input={"query":{}}
```

Output includes inline node trace:
```
--- node trace (2 nodes, 8ms total) ---
  ✓  n0  (trigger.webhook)   0ms
  ✓  n1  (pg.query)          8ms
```

If a node shows `✗`, inspect that node ID and fix before moving on.

---

## Phase 7: Commit + Handoff

```
git_command  subcommand=add      args="."
git_command  subcommand=commit   message="feat: complete blog v1 — list, post detail, admin CRUD"
```

Agent updates MEMORY.md:

```
docs_agent_write
  name=MEMORY.md
  content="""
  # Session 1 — Blog v1 Complete

  ## Delivered
  - 5 pipelines (all active): blog-list, blog-post, admin-posts, admin-post-get, admin-post-put
  - 4 templates: blog-list, blog-post, admin-posts, admin-editor
  - Docs: REQUIREMENTS.md, schema.md, architecture.md

  ## Prerequisites (owner must do)
  - Create `posts` table in PostgreSQL (schema in docs/schema.md)

  ## Known gaps for v2
  - No pagination on blog-list (currently LIMIT 20)
  - Auth uses HTTP Basic — consider session tokens for v2
  - No image upload support
  - About page not built (low priority)

  ## Routes
  Public:  GET /blog,  GET /blog/post?slug=...
  Admin:   GET /admin/posts,  GET /admin/post?slug=...  (Basic auth)
  """
```

---

## Key Patterns Demonstrated

| Pattern | Where shown |
|---------|-------------|
| Write spec before code | Phase 1–3: REQUIREMENTS.md, schema.md, architecture.md |
| Data flows from pipeline to template via `input.state` | Phase 4 pipelines → Phase 5 `input.state?.rows` |
| Auth in a script node before query | admin-posts pipeline |
| `hydration: "static"` for read-only pages | blog-list, blog-post, admin-posts |
| `hydration: "reactive"` for interactive forms | admin-editor |
| `useState` hooks for form binding | admin-editor component |
| Always commit after logical chunk | Phase 6 |
| Always update MEMORY.md before ending session | Phase 7 |
| Design system components only — no raw HTML | Button, Input, Field, Label, Badge |
| Test with `pipeline_execute` to see node trace | Phase 6 verification |
| Use `pipeline_get_invocations` to debug scheduled runs | (when scheduler involved) |
