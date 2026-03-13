# Zebflow Platform Overview

## Architecture

Zebflow is a pipeline-based reactive web automation platform. Projects contain:

- **Pipelines** (`.zf.json`) — JSON-defined directed graphs connecting trigger nodes to action nodes
- **Templates** (`.tsx`) — TSX-based server-side rendered UI using the RWE engine
- **Simple Tables** — Managed key-value rows backed by Sekejap, queryable from pipelines
- **Credentials** — Encrypted secrets (API keys, DB passwords) referenced by pipeline nodes
- **DB Connections** — Named connections to PostgreSQL or SjTable databases
- **Agent Docs** — AGENTS.md, SOUL.md, MEMORY.md for agent session continuity

## Project Structure

```
{project-root}/
├── repo/
│   ├── zebflow.json          ← project config
│   ├── pipelines/            ← .zf.json pipeline definitions
│   ├── templates/
│   │   ├── pages/            ← full-page TSX templates
│   │   ├── components/       ← reusable TSX components
│   │   │   ├── ui/           ← design system (always use)
│   │   │   ├── layout/       ← page shell wrappers
│   │   │   └── behavior/     ← client-side behavior modules
│   │   ├── scripts/          ← shared TS utility modules
│   │   └── styles/           ← CSS files
│   └── docs/                 ← project documentation
├── data/
│   └── sekejap/              ← project data + agent docs
└── files/                    ← public static assets
```

## Key Concepts

**Owner**: User/organization identifier. Also the namespace for projects.

**Project**: Isolated workspace. Each project has its own pipelines, templates, tables, credentials.

**Pipeline**: A graph of nodes. Trigger nodes receive HTTP requests or cron ticks; action nodes transform data; web_render nodes produce HTML output.

**Activation**: Pipelines must be explicitly activated to serve live traffic. Draft changes don't affect production until activated.

**MCP Session**: A scoped token granting an LLM agent access to a project's tools. Created by a project owner, usable from Cursor, Claude, or any MCP-compatible client.

## API Base

All project APIs: `POST/GET /api/projects/{owner}/{project}/...`

Webhook ingress: `{method} /wh/{owner}/{project}/{webhook-path}`
