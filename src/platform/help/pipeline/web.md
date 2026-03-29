# Pipeline DSL — Web responses (`n.web.response`)

This doc covers **serving HTTP responses** from pipelines: HTML pages, JSON, redirects, cookies, and custom headers — all via `n.web.response`.

See also: **`pipeline-dsl`** (full DSL), **`web-templates`** (how to write `.tsx` pages).

---

## What `n.web.response` does

Without `--template`: serves the upstream payload as **JSON** (status 200 by default).
With `--template`: compiles the TSX file, renders it to **HTML** on the server (SSR), and hydrates on the client.

All HTTP concerns (status, cookies, headers, redirects) are explicit flags — nothing hidden.

---

## DSL flags

| Flag | Description |
|------|-------------|
| `--template pages/foo.tsx` | TSX page to render. Activates RWE mode — upstream payload becomes `input` in the template. |
| `--status 404` | HTTP status code (default: 200, or 302 when `--location` is set). |
| `--location /path` | Redirect URL. Implies 302 unless `--status` overrides. Supports `$.field` to resolve from upstream payload (e.g. `--location $.redirect_url`). |
| `--message "text"` | Plain-text response body. |
| `--body $.field` | JSON path into upstream payload to use as response body. |
| `--set-cookie spec` | Set a cookie — see spec format below. |
| `--header K=V` | Extra response header. Repeatable. |
| `--load-scripts url` | External script URLs to inject (template mode only, comma-separated). |

---

## Cookie spec format (`--set-cookie`)

Comma-separated key=value pairs:

```
name=session,value=$.access_token,http-only,max-age=86400,secure,same-site=Strict,path=/
```

| Part | Meaning |
|------|---------|
| `name=NAME` | Cookie name (required). |
| `value=$.path` | Cookie value — `$.field` resolves from upstream payload, or use a literal. |
| `http-only` | Sets HttpOnly flag (default: on). |
| `secure` | Sets Secure flag. |
| `max-age=SECS` | Max-Age directive (default: 900). |
| `same-site=Lax` | SameSite (default: Lax). |
| `path=/` | Cookie path (default: /). |

---

## Patterns

### Serve JSON (no template)

```zf
| trigger.webhook --path /api/posts --method GET
| pg.query --credential main-db -- "SELECT id, title FROM posts"
| web.response
```

### Render an HTML page

```zf
| trigger.webhook --path /blog --method GET
| pg.query --credential main-db -- "SELECT id, title, published_at FROM posts ORDER BY published_at DESC LIMIT 20"
| web.response --template pages/blog-home.tsx
```

### 404 error page

```zf
| trigger.webhook --path /blog/:id --method GET
| pg.query --credential main-db -- "SELECT * FROM posts WHERE id = $1"
| script -- "if (!input.rows?.[0]) return { __notfound: true }; return input.rows[0]"
| web.response --template pages/not-found.tsx --status 404
```

### Redirect — static URL

```zf
| trigger.webhook --path /go/signup --method GET
| web.response --location /auth/register
```

### Redirect — dynamic URL from payload

`$.field` resolves the redirect target from the upstream payload at execution time.

```zf
| trigger.webhook --path /auth/login --method POST
| pg.query --credential main-db -- "SELECT dashboard_url FROM users WHERE email = $1"
| script -- "return { redirect_url: input.rows?.[0]?.dashboard_url ?? '/home' }"
| web.response --location $.redirect_url
```

### Login — set session cookie

```zf
| trigger.webhook --path /auth/login --method POST
| pg.query --credential main-db -- "SELECT id, role FROM users WHERE email = $1"
| auth.token.create --credential my-jwt --claim sub=$.rows[0].id --claim role=$.rows[0].role
| web.response --template pages/home.tsx --set-cookie name=session,value=$.access_token,http-only,max-age=86400
```

### Custom headers

```zf
| trigger.webhook --path /api/data --method GET
| pg.query --credential main-db -- "SELECT * FROM data"
| web.response --header Content-Type=application/json --header X-Version=2
```

---

## Accessing server data in TSX templates

The upstream pipeline payload becomes **`input`** (the function parameter) inside the template. `ctx` is the same object available as a global (`globalThis.ctx`) in both SSR and browser.

```tsx
export default function Page(input) {
  // input = full upstream payload (e.g. { rows: [...], total: 42 })
  const posts = input?.rows ?? [];
  const [selected, setSelected] = useState(null);

  return (
    <Page>
      <main>
        {posts.map(p => <div key={p.id}>{p.title}</div>)}
      </main>
    </Page>
  );
}
```

**Rules:**
- Use `input` (function parameter) to access server data — works in both SSR and browser.
- Use `useState`, `useEffect`, etc. for client interactivity — these are globals, no import needed.
- `ctx` also works as a bare global if you prefer, but `input` as the function param is the convention.

---

## Where templates live

`repo/pipelines/` — e.g. `pages/...`, `components/...`, `shared/ui/...`. Imports use **`@/`** from that root.
