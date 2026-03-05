# Zebflow Platform Overview

## Architecture

Zebflow is a visual pipeline orchestration platform. Projects contain:

- **Pipelines** (`.zf.json`) — JSON-defined directed graphs connecting trigger nodes to action nodes
- **Templates** (`.tsx`) — TSX-based server-side rendered UI components using the RWE engine
- **Simple Tables** — Managed key-value rows backed by Sekejap, queryable from pipelines
- **Credentials** — Encrypted secrets (API keys, DB passwords) referenced by pipeline nodes
- **DB Connections** — Named connections to PostgreSQL or SjTable databases

## Project Structure

```
app/
  pipelines/          # .zf.json pipeline definitions
  templates/
    pages/            # Full-page TSX templates
    components/       # Reusable TSX components
    scripts/          # Shared TS utility modules
  docs/               # Project documentation (README.md, AGENTS.md, ERD, etc.)
```

## Key Concepts

**Owner**: User/organization identifier. Also the namespace for projects.

**Project**: Isolated workspace. Each project has its own pipelines, templates, tables, credentials.

**Pipeline**: A graph of nodes. Webhook nodes receive HTTP requests; script/query nodes transform data; web_render nodes produce HTML output.

**Activation**: Pipelines must be explicitly activated to serve live traffic. Draft changes don't affect production until activated.

**MCP Session**: A scoped token granting an LLM agent access to a project's tools. Created by a project owner, usable from Cursor/Claude.

## API Base

All project APIs are at: `POST/GET /api/projects/{owner}/{project}/...`

Webhook ingress: `{method} /wh/{owner}/{project}/{webhook-path}`
