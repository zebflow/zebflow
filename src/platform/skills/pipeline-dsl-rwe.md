# Pipeline DSL — RWE & web.render

**RWE (Reactive Web Engine)** is Zebflow's core feature: instantly register reactive web pages
that are server-side rendered and client-side hydrated. `n.web.render` is the node that powers this.

See also: **pipeline-dsl** (main DSL reference), **rwe-templates** (template authoring).

---

## What web.render Does

`n.web.render` maps a pipeline to a live HTTP route. When a GET request arrives:

1. The pipeline runs (trigger fires).
2. Prior nodes compute data → passed as `props` to the template.
3. The TSX template renders to HTML (SSR).
4. The page is returned, optionally hydrated for client-side reactivity.

Result: **a reactive web page with zero boilerplate**.

---

## Registering a web.render Pipeline

```zf
register blog-home --path /pages \
  | trigger.webhook --path /blog --method GET \
  | pg.query --credential my-db \
    -- "SELECT id, title, published_at FROM posts ORDER BY published_at DESC LIMIT 20" \
  | web.render --template blog-home --route /blog
```

This creates:
- A GET route at `/blog`
- Queries PostgreSQL on every request
- Renders `blog-home` template with query results as props

---

## web.render Config

```zf
n.web.render --help
```

| Flag | Required | Description |
|---|---|---|
| `--template <id>` | yes | Template slug (filename without extension) |
| `--route <path>` | yes | URL path to serve the page at |
| `--markup <html>` | no | Inline HTML override (rare) |

---

## Template Structure

Templates live in `repo/templates/` as `.tsx` files. The RWE compiler:

1. Strips `from "rwe"` import lines (hooks are injected as globals)
2. Compiles TSX → JS
3. Runs SSR in Node.js worker
4. Injects hydration script for client-side reactivity

### Minimal template

```tsx
export default function BlogHome(props) {
  const posts = props?.posts ?? [];

  return (
    <div className="blog-home">
      <h1>Blog</h1>
      <ul>
        {posts.map((p) => (
          <li key={p.id}>
            <a href={`/blog/${p.id}`}>{p.title}</a>
            <time>{p.published_at}</time>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

### Reactive template (client hooks)

Hooks are globals — no imports needed:

```tsx
export default function Counter(props) {
  const [count, setCount] = useState(props?.initial ?? 0);

  return (
    <div>
      <p>Count: {count}</p>
      <button onClick={() => setCount(count + 1)}>+</button>
    </div>
  );
}
```

Available globals: `useState`, `useEffect`, `useRef`, `useMemo`, `usePageState`.

### usePageState — SSE-backed live state

`usePageState` connects to an SSE endpoint and keeps state live:

```tsx
export default function Dashboard(props) {
  const [stats, setStats] = usePageState("stats", props?.stats ?? {});

  return (
    <div>
      <p>Active users: {stats.active_users ?? 0}</p>
    </div>
  );
}
```

---

## Props from Pipeline

The pipeline result object becomes `props` for the template.

If a `pg.query` node returns:
```json
{ "rows": [{ "id": 1, "title": "Hello" }] }
```

Then in the template:
```tsx
const posts = props?.rows ?? [];
```

For multi-node pipelines, each node's output is merged into `input` for the next node.
The final node's output is the template's `props`.

---

## Layouts and Components

Templates can import other templates as components:

```tsx
import PlatformSidebar from "@/components/platform-sidebar";
import BlogPostCard from "@/components/blog-post-card";

export default function BlogHome(props) {
  return (
    <div className="blog-layout">
      <PlatformSidebar />
      <main>
        {(props?.posts ?? []).map((p) => <BlogPostCard key={p.id} post={p} />)}
      </main>
    </div>
  );
}
```

`@/` resolves to `repo/templates/`.

---

## SSR + Hydration Modes

| Mode | Behavior |
|---|---|
| SSR only | Full HTML, no JS sent. Fast, no interactivity. |
| SSR + hydrate | HTML + hydration script. Full reactivity. |
| Client-only | Blank shell, JS renders everything. (Rare) |

Default is SSR + hydrate when hooks are used, SSR-only otherwise.

---

## Complete Example: Blog with DB + Reactive Like Counter

```zf
# Register the blog post page
register blog-post --path /pages \
  | trigger.webhook --path /blog/:id --method GET \
  | pg.query --credential main-db \
    -- "SELECT * FROM posts WHERE id = $1" \
  | web.render --template blog-post --route /blog/:id
```

Template `repo/templates/pages/blog-post.tsx`:

```tsx
import BlogLayout from "@/components/layout/blog-layout";

export default function BlogPost(props) {
  const post = props?.rows?.[0] ?? {};
  const [likes, setLikes] = useState(post.likes ?? 0);

  async function handleLike() {
    await fetch(`/api/posts/${post.id}/like`, { method: "POST" });
    setLikes(likes + 1);
  }

  return (
    <BlogLayout title={post.title}>
      <article>
        <h1>{post.title}</h1>
        <p>{post.body}</p>
        <button onClick={handleLike}>Like ({likes})</button>
      </article>
    </BlogLayout>
  );
}
```
