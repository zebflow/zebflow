# Full project workflow — from a brief to a live site

A worked example of one agent session over MCP: **a small blog with public
pages and a signed-in admin**. Every step is a tool call; every claim of
"done" is a fetch. The skills named here (`skill_read name="…"`) carry the
procedure for each kind of task; this page shows them in order.

---

## Phase 0 — orient

```
start_here
docs_agent_read  name=AGENTS.md
docs_agent_read  name=MEMORY.md
pipeline_list
file_list
connection_list
credential_list
```

On a fresh project the agent learns: no pipelines, a sample page, the two
built-in connections (`default` SQLite, `default-multimodel` Sekejap), and
one credential the owner made for sign-in — `jwt_main`, kind
`jwt_signing_key`. A credential id is what `--credential` takes; a
connection slug is only for `connection_describe`.

Before writing, it reads what this task needs:

```
skill_read  name=zebflow-basic
skill_read  name=zebflow-engineering
help        topic="pipeline/dsl"
skill_read  name=zebflow-pipeline
```

---

## Phase 1 — the brief and the layout

The agent writes down what it will build before building it, so the owner
can correct it and the next session can read it:

```
file_write  rel_path=docs/brief.md  content="""
# Blog
Public: post list at /blog, a post at /blog/:slug.
Admin (role admin): post list at /admin/posts, a form that creates a post.
Data: Sekejap table posts (title, slug, body, status, created_at).
Auth: --auth jwt --credential jwt_main on every /admin route, pages and POSTs.
"""
file_write  rel_path=docs/structure.md  content="""
Flat layout: pages/ (pipelines + templates), api/ (POST pipelines), components/, db/.
"""
```

---

## Phase 2 — the data

```
skill_read  name=zebflow-data
skill_read  name=zebflow-sekejap
connection_describe  slug=default-multimodel  scope=tables
file_write  rel_path=db/001_posts.sql  content="CREATE TABLE posts (title TEXT, slug TEXT, body TEXT, status TEXT, created_at TEXT)"
pipeline_run  body="| trigger.function | sekejap.query.run --write -- \"CREATE TABLE posts (title TEXT, slug TEXT, body TEXT, status TEXT, created_at TEXT)\""
connection_describe  slug=default-multimodel  table=posts
```

The migration is a file first and a statement second; `connection_describe`
reads it back. `docs/schema.md` records the columns and what `status` may be.

---

## Phase 3 — the pipelines

Each route is a pipeline and, when it renders, the template it names. Every
node adds one key to the payload: after the query, the rows are
`input.query.rows` — in the next node and in the page.

**The post list**

```
pipeline_register  file_rel_path=pages/blog  title="Blog"  body="""
| trigger.webhook --route /blog --method GET
| sekejap.query.run -- "SELECT title, slug, created_at FROM posts WHERE status = 'published' ORDER BY created_at DESC LIMIT 20"
| web.response.send --template pages/blog.tsx
"""
```

**One post, or a 404**

```
pipeline_register  file_rel_path=pages/blog-post  title="Blog post"  body="""
[a] trigger.webhook --route /blog/:slug --method GET
[b] sekejap.query.run --param "1={{ $trigger.params.slug }}" -- "SELECT title, body, created_at FROM posts WHERE slug = $1 AND status = 'published'"
[c] logic.if --when "input.query.rows.length > 0"
[d] web.response.send --template pages/blog-post.tsx
[e] web.response.send --status 404 --template pages/not-found.tsx
[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
"""
```

**The admin list** — the trigger refuses anyone without the `admin` role
before a node runs; a browser without a session is sent to the
credential's `auth_redirect`.

```
pipeline_register  file_rel_path=pages/admin-posts  title="Admin posts"  body="""
| trigger.webhook --route /admin/posts --method GET --auth jwt --credential jwt_main --role admin
| sekejap.query.run -- "SELECT title, slug, status, created_at FROM posts ORDER BY created_at DESC LIMIT 100"
| web.response.send --template pages/admin-posts.tsx
"""
```

**Creating a post** — the form's POST: the same auth, a check, the insert
with every value bound through `--param`, then a redirect back to the list.

```
pipeline_register  file_rel_path=api/admin-posts-create  title="Create post"  body="""
[a] trigger.webhook --route /admin/posts --method POST --auth jwt --credential jwt_main --role admin
[b] logic.if --when "typeof input.webhook.body?.title === 'string' && input.webhook.body.title.trim().length > 0"
[c] sekejap.query.run --write --param "1={{ $trigger.body.title }}" --param "2={{ $trigger.body.title.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-') }}" --param "3={{ $trigger.body.body ?? '' }}" --param "4={{ $trigger.body.status === 'published' ? 'published' : 'draft' }}" --param "5={{ new Date().toISOString() }}" -- "INSERT INTO posts (title, slug, body, status, created_at) VALUES ($1, $2, $3, $4, $5)"
[d] web.response.send --status 303 --header "Location=/admin/posts"
[e] web.response.send --status 303 --header "Location=/admin/posts?error=title"
[a] -> [b]
[b]:true -> [c]
[c] -> [d]
[b]:false -> [e]
"""
```

All four are `draft` until activated.

---

## Phase 4 — the pages

```
skill_read  name=zebflow-rwe
skill_read  name=zebflow-ui
list_ui_catalog
```

Every page imports what it uses and colours with theme roles. The list:

```
file_write  rel_path=pages/blog.tsx  content="""
import "@/globals.css";

export default function Blog(input) {
  const posts = input.query?.rows ?? [];
  return (
    <main className="mx-auto max-w-2xl px-4 py-12">
      <h1 className="text-3xl font-semibold text-foreground">Blog</h1>
      {posts.length === 0 && <p className="mt-6 text-muted-foreground">No posts yet.</p>}
      <ul className="mt-8 space-y-4">
        {posts.map((p) => (
          <li key={p.slug}>
            <a href={`/blog/${p.slug}`} className="text-lg font-medium text-primary hover:underline">{p.title}</a>
            <p className="text-sm text-muted-foreground">{p.created_at?.slice(0, 10)}</p>
          </li>
        ))}
      </ul>
    </main>
  );
}

export const page = { head: { title: "Blog" } };
"""
```

The admin list carries the form; it posts to the route above and the page
re-renders from the redirect:

```
file_write  rel_path=pages/admin-posts.tsx  content="""
import { Badge } from "zeb/ui/badge";
import { Button } from "zeb/ui/button";
import { Input } from "zeb/ui/input";
import { Textarea } from "zeb/ui/textarea";
import { Field, FieldLabel } from "zeb/ui/field";
import "@/globals.css";

export default function AdminPosts(input) {
  const posts = input.query?.rows ?? [];
  const missingTitle = input.webhook?.query?.error === "title";
  return (
    <main className="mx-auto max-w-3xl space-y-10 px-4 py-10">
      <form method="post" action="/admin/posts" className="space-y-4">
        <Field>
          <FieldLabel htmlFor="title">Title</FieldLabel>
          <Input id="title" name="title" required />
          {missingTitle && <p className="text-sm text-destructive">A post needs a title.</p>}
        </Field>
        <Field>
          <FieldLabel htmlFor="body">Body</FieldLabel>
          <Textarea id="body" name="body" rows={8} />
        </Field>
        <label className="flex items-center gap-2 text-sm text-foreground">
          <input type="checkbox" name="status" value="published" /> Publish now
        </label>
        <Button type="submit">Create post</Button>
      </form>
      <ul className="divide-y divide-border">
        {posts.map((p) => (
          <li key={p.slug} className="flex items-center justify-between py-3">
            <span className="text-foreground">{p.title}</span>
            <Badge variant={p.status === "published" ? "default" : "secondary"}>{p.status}</Badge>
          </li>
        ))}
      </ul>
    </main>
  );
}

export const page = { head: { title: "Posts — admin" } };
"""
```

`pages/blog-post.tsx` reads `input.query.rows[0]`; `pages/not-found.tsx` is
the empty state with a link home. Each `file_write` reports a compiler
refusal (a hook used without its import, a disallowed import) at once — fix
it before going on.

---

## Phase 5 — activate and prove

```
pipeline_activate  glob="pages/**"
pipeline_activate  file_rel_path=api/admin-posts-create
pipeline_list      status=all                                     ← all four active
```

```
skill_read   name=zebflow-verify
route_fetch  path="/blog"                                          ← 200, no rwe_component_errors, "No posts yet."
route_fetch  path="/admin/posts"                                   ← 303 to the auth_redirect: no session
route_fetch  path="/admin/posts" cookie="zebflow_session=…"        ← 200 with a session the login gave you
route_fetch  path="/admin/posts" method=POST form={"title":"Hello","status":"published"} cookie="zebflow_session=…"   ← 303 to /admin/posts
route_fetch  path="/blog/hello"                                    ← 200, "Hello" in the body
route_fetch  path="/blog/missing"                                  ← 404 page
pipeline_get_invocations  file_rel_path=api/admin-posts-create     ← one clean run, the nodes you expected
```

Then open `/admin/posts` in a browser with the console visible, submit the
form, and check the new row appears — server HTML says nothing about
hydration.

---

## Phase 6 — record and hand over

```
git_command  subcommand=add     args="."
git_command  subcommand=commit  message="feat: blog list, post page, admin create"
docs_agent_write  name=MEMORY.md  content="""
Built: /blog, /blog/:slug (404 when missing), /admin/posts (list + create form, role admin).
Verified: route_fetch on each route and the failure paths; invocation of api/admin-posts-create clean.
Open: editing and deleting posts; pagination after 20 posts.
"""
```

| Pattern | Where |
|---|---|
| a written brief and layout before code | Phase 1 |
| migrations are files, read back with `connection_describe` | Phase 2 |
| one answer key per node: `input.query.rows` from the query to the page | Phase 3, 4 |
| auth on the trigger, on the page **and** its POST | Phase 3 |
| values bound through `--param`, a write marked `--write` | Phase 3 |
| a form is two pipelines; the POST redirects with `--status 303 --header "Location=…"` | Phase 3, 4 |
| theme roles and `zeb/ui` components, every import explicit | Phase 4 |
| done means fetched, failure paths included | Phase 5 |
