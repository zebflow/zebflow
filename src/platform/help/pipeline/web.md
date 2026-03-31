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
| script -- "const u = input.rows[0]; return { ...u, roles: [u.role] }"
| auth.token.create --credential my-jwt --claim sub=$.id --claim roles=$.roles:public
| web.response --template pages/home.tsx --set-cookie name=session,value=$.access_token,http-only,max-age=86400
```

> `roles` claim must be an array. Wrap a single DB `role` string with `[u.role]` in a script node before signing.

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

## Trigger context in templates — `ctx.auth`, `ctx.params`, `ctx.query`, `ctx.headers`

`n.web.response` **always injects trigger fields into the template state**, regardless of what upstream nodes did to the payload. Even when `pg.query` replaces `input` with `{rows:[...]}`, the template still sees:

| Template field | Source | Description |
|---|---|---|
| `ctx.auth` | `trigger.auth` (after public-claim filter) | Verified JWT claims. Only claims marked `:public` when issued. `null` if no public claims or no JWT. |
| `ctx.params` | `trigger.params` | URL path params (`:id`, `:slug`, etc.) |
| `ctx.query` | `trigger.query` | Query string params (`?page=2` etc.) |
| `ctx.headers` | `trigger.headers` | Safe request headers (content-type, user-agent, etc.) |

These fields are injected by `inject_trigger_fields()` just before the template renders, and they come from `metadata["trigger"]` — the immutable trigger snapshot that flows through every node unchanged.

### The `_zf_public` mechanism — what `ctx.auth` contains

When a JWT is issued via `auth.token.create`, only claims explicitly marked `:public` are visible in the browser. Claims without `:public` are signed into the JWT but stripped before reaching the DOM.

```zf
# Only name and role reach the browser as ctx.auth.name and ctx.auth.role
| auth.token.create --credential my-jwt \
    --claim sub=$.id \
    --claim name=$.fullname:public \
    --claim role=$.role:public \
    --claim internal_id=$.db_id   ← never visible in browser
```

**Effect in templates:**
```tsx
export default function Page(input) {
  // ctx is always available as a global
  const userName = ctx.auth?.name ?? 'Guest';   // only if marked :public
  const userRole = ctx.auth?.role ?? null;
  const userId = ctx.auth?.sub;                 // NOT available — sub not marked :public
  ...
}
```

If **no claims are marked `:public`**, `ctx.auth` is `null` even for authenticated users. This is intentional — secure by default.

### Why this survives payload replacement

```
trigger.webhook --auth-type jwt  →  auth verified; trigger.auth = decoded claims
pg.query                         →  payload becomes { rows: [...] } — auth is gone from input
script                           →  transforms rows — auth still gone from input
web.response --template ...      →  inject_trigger_fields() restores auth/params/query/headers
                                    into state before rendering
```

The template always has `ctx.auth`, `ctx.params`, `ctx.query`, `ctx.headers` — no matter how many nodes transformed the payload between the trigger and the response.

### Example — auth-aware template

```tsx
export default function Dashboard(input) {
  // input = whatever pg.query returned — { rows: [...] }
  // ctx.auth comes from the original JWT, not from input
  const user = ctx.auth;  // { name: "Alice", role: "admin" } — public claims only
  const params = ctx.params;  // { id: "42" } — from /dashboard/:id
  const query = ctx.query;    // { tab: "overview" } — from ?tab=overview

  if (!user) return <div>Not authenticated</div>;

  return (
    <main>
      <h1>Hello {user.name}</h1>
      <p>Role: {user.role}</p>
      {input.rows.map(r => <div key={r.id}>{r.title}</div>)}
    </main>
  );
}
```

---

## Dynamic expressions in `n.web.response` flags

`{{ expr }}` is supported in `--location`, `--set-cookie value=...`, and `--header` values, resolved from the pipeline payload just before the response is sent.

```zf
# Redirect to a URL built from trigger params and upstream data
| web.response --location "/users/{{ $trigger.params.id }}/{{ $nodes.lookup.rows[0].slug }}"

# Set a cookie whose value comes from auth.token.create output
| web.response --set-cookie "name=session,value={{ $input.access_token }},http-only,max-age=86400"

# Inject a custom header with the authenticated user's ID
| web.response --header "X-User-Id={{ $trigger.auth.sub }}"
```

See `help(topic="pipeline/dsl")` for the full `{{ expr }}` scope reference.

---

## Where templates live

`repo/pipelines/` — e.g. `pages/...`, `components/...`, `shared/ui/...`. Imports use **`@/`** from that root.
