# Zebflow Agent Core

Zebflow is a pipeline-based reactive web automation platform. Pipelines connect nodes to produce web endpoints, REST APIs, scheduled jobs, and browser automation. The RWE (Reactive Web Engine) compiles TSX + TypeScript on the fly, SSR-renders pages with live pipeline data, and hydrates them client-side — no build step, no deploy.

Your single tool is `execute_pipeline_dsl`. Everything goes through it.

---

## Hello World

### REST API

```
register hello-api --path /api \
  | trigger.webhook --path /hello --method GET \
  | script -- "return { message: 'Hello', ts: Date.now() }"
activate pipeline hello-api
```

### Web Page (webhook → DB → render)

```
register blog-home --path /pages \
  | trigger.webhook --path /blog --method GET \
  | pg.query --credential main-db -- "SELECT id, title, created_at FROM posts ORDER BY created_at DESC LIMIT 20" \
  | web.render --template blog-home --route /blog
activate pipeline blog-home
```

Template `repo/templates/pages/blog-home.tsx`:
```tsx
export default function BlogHome(props) {
  const posts = props?.rows ?? [];
  return (
    <div className="p-8 bg-slate-950 text-slate-100 min-h-screen">
      <h1 className="text-2xl font-bold mb-6">Blog</h1>
      {posts.map(p => (
        <div key={p.id} className="mb-4 p-4 bg-slate-900 rounded-lg">
          <a href={`/blog/${p.id}`} className="text-sky-400 hover:underline">{p.title}</a>
          <time className="text-slate-500 text-sm ml-4">{p.created_at}</time>
        </div>
      ))}
    </div>
  );
}
```

### Scheduled Task

```
register daily-digest --path /jobs \
  | trigger.schedule --cron "0 8 * * *" --timezone "Asia/Jakarta" \
  | pg.query --credential main-db -- "SELECT * FROM events WHERE date = CURRENT_DATE" \
  | http.request --url https://hooks.slack.com/xxx --method POST
activate pipeline daily-digest
```

---

## Knowledge Docs

### Core Skills

| Command | Covers |
|---|---|
| `read skill pipeline-dsl` | All DSL commands — register, run, activate, git, connections, nodes, graph mode, logic branching |
| `read skill rwe-templates` | TSX templates, Tailwind, tw-variants, component libraries, hydration, typed class slots |
| `read skill sekejapql` | SjTable queries, create table, upsert rows, all filter operators |

Use `get skills` to list all available docs.
Use `--help` on any node or command for inline usage: `pg.query --help`, `trigger.webhook --help`, `sekejap.query --help`.

### DSL Cheat Sheet
