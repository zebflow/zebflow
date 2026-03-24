# zebflow

**[zebflow.com](https://zebflow.com)** · [Documentation](docs/README.md)

> **Ship once. Build forever.**

Zebflow is an open-source platform for building and serving full-stack reactive web apps — without a build toolchain. Write pipelines, render React pages (SSR + SPA), add real-time WebSocket rooms, connect databases, and run scheduled jobs — all from a single binary or Docker container.

Keep building from your laptop: connect your IDE or any MCP-compatible AI agent directly to the running instance. Changes go live instantly and sync to git automatically.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-~450MB-informational.svg)](https://hub.docker.com/r/zebflow/zebflow)

---

## What it does

- **Full-stack React, no build step** — write TSX components directly in the app. They compile and render server-side, live. No `npm install`, no Webpack, no Vite.
- **Pipeline automation** — connect webhooks, databases, HTTP calls, schedules, script transforms, and AI agents in a pipe-chained DSL or visual editor.
- **SSR + SPA out of the box** — pages are server-rendered, hydrated on the client with Preact. Navigation is intercepted client-side for instant transitions. No configuration.
- **Real-time WebSocket rooms** — built-in multi-client rooms with shared state. Trigger pipelines on WS events, sync state, broadcast to all or targeted sessions.
- **Build from your laptop via MCP** — connect Claude Code, Cursor, or any MCP client to the running instance. Create pipelines, write templates, query data — changes go live without touching the server. Everything commits to git.
- **Git-synced by default** — every pipeline and template is a file on disk, version-controlled and reviewable.

---

## Who it's for

You have a running server. You want to build internal tools, dashboards, or lightweight web apps on top of your data — without spinning up a separate frontend project, CI pipeline, or build server.

Zebflow runs alongside your stack. You write a TSX page, wire it to a database query in a pipeline, and it's live at a URL. You keep iterating from wherever you are — browser, terminal, or IDE.

---

## Quick start

```bash
docker run -p 10610:10610 zebflow/zebflow
```

Open `http://localhost:10610` — default login: `superadmin` / `admin`.

---

## How development works

1. **Create a pipeline** — write the DSL in the in-app console or use the visual editor. One line connects a trigger to a query to a rendered page.
2. **Write a React page** — open the template editor, write TSX, save — it's live at the pipeline's path immediately.
3. **Connect your data** — add a database credential, drop a query node before the render node. The query result flows in as the page's `input` prop.
4. **Keep building from anywhere** — connect your IDE or AI agent via MCP. Register pipelines, write templates, commit to git — all from your laptop, without SSH or redeploy.

No rebuild. No redeploy. The running instance is the development environment.

---

## Stack

| Layer | What it is |
|-------|-----------|
| Runtime | Rust — single binary, ~90 MB Docker image |
| Script engine | Embedded V8 via `deno_core` — no Node.js required |
| React rendering | TSX → SSR via embedded Deno, hydrated with Preact |
| Pipelines | 20+ built-in nodes: triggers, HTTP, SQL, WebSocket, AI, logic, web render |
| Data | Sekejap (embedded graph-first multimodel db engine), PostgreSQL, MySQL, simple tables |
| Real-time | WebSocket rooms with shared state, event dispatch, broadcast |
| AI | BYOK — OpenAI-compatible or Anthropic, MCP protocol |

---

## Documentation

- [Getting Started](docs/GETTING_STARTED.md)
- [Pipeline Reference](docs/PIPELINES.md)
- [RWE — Reactive Web Engine](docs/RWE.md)
- [MCP Integration](docs/MCP.md)
- [Platform Web](docs/developer-guide/platform-web.md)

---

## License

MIT — see [LICENSE](LICENSE).
