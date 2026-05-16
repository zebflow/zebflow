# zebflow

**[zebflow.com](https://zebflow.com)** · [Docs Index](docs/README.md) · [User Guide](docs/user-guide/README.md) · [Dev Guide](docs/dev-guide/README.md)

> Full-stack reactive web automation.

Zebflow is a full-stack reactive web automation platform. Build SSR pages, SPA flows, APIs, real-time rooms and games, and various automations in one running system using Pipelines and Templates.

Deploy once, evolve continuously. Write TSX directly in the running instance, compose behavior with pipelines, and build reactive frontend on the fly without a separate frontend build toolchain.

It is designed to grow from one local office to multi-user, multi-project, multi-office deployments. MCP and BYOK agent console flows are first-class at project scope.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-insanalamin%2Fzebflow-informational.svg)](https://hub.docker.com/r/insanalamin/zebflow)

## Overview

Zebflow combines two authoring surfaces:

- **Templates**
  TSX pages and components rendered through the Reactive Web Engine without a separate build step.
- **Pipelines**
  Triggered automation graphs for webhooks, functions, schedules, data access, HTTP, AI, state, and rendering.

Those two surfaces stay inside the same project workspace, so a project can serve:

- SSR pages
- SPA-like navigation
- JSON or form APIs
- webhook handlers
- WebSocket or realtime room flows
- static generated outputs
- operator and agent automation

## Walkthroughs

Add short GIF walkthroughs here before the next public-facing release:

1. create a page and pipeline in Project Studio
2. connect MCP or agent console to one project
3. place a project onto a remote office

## Install

### Docker

Current stable install path:

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
docker run --name zebflow \
  -p 10610:10610 \
  -v zebflow-data:/var/lib/zebflow/data \
  -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD \
  insanalamin/zebflow:latest
```

Then open:

- `http://localhost:10610/login`

### Source Build

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
cargo run --bin zebflow
```

### Other Distribution Channels

The following channels are planned, but they are not published as stable install
channels yet:

- `npm install zebflow`
- `pip install zebflow`
- single-binary release downloads

The README keeps them separate on purpose so install instructions stay honest.

## Runtime Modes

One binary, different roles:

```bash
zebflow
zebflow controller
zebflow office
```

- `zebflow`
  standalone controller + office
- `zebflow controller`
  management-oriented office
- `zebflow office`
  execution-oriented office joined under a managing office

## Features

- **Reactive Web Engine**
  Write TSX directly in the running project. SSR, SPA-style navigation, client hydration, and library loading happen without a separate Vite/Webpack-style build.
- **Pipeline + Template composition**
  Build web pages, APIs, webhooks, scheduled jobs, functions, and automation from the same project workspace.
- **Agentic tools from project nodes and functions**
  Project functions and pipeline nodes act as tool surfaces for agentic work. The built-in agent node supports two modes: `direct` for immediate task execution and `strategic` for higher-level planning and delegated reasoning.
- **Real-time runtime**
  Native WebSocket rooms and event-driven flows for collaborative tools, dashboards, simulations, or realtime game mechanics.
- **Static generation into project files**
  Generate durable outputs into `files/public` or `files/private` from pipeline execution.
- **Project-scoped MCP and agent console**
  Each project can expose MCP and BYOK assistant workflows as first-class management surfaces.
- **Git-synced workspace**
  Source lives on disk, stays reviewable, and can be exported, imported, and versioned as a real project workspace.
- **Installable libraries, UI components, and examples**
  The running system can install reusable project assets instead of forcing every project to bootstrap from scratch.
- **Multi-user, multi-project operation**
  One office can host multiple projects and multiple operators while preserving project scope.
- **Office federation**
  Zebflow can scale from one office to controller-plus-office federation and future multi-office deployments.
- **Single binary deployment model**
  The same binary is intended to work on a laptop, Raspberry Pi, one server, or Kubernetes with role-based configuration.

## Documentation

- [Docs Index](docs/README.md)
- [User Guide](docs/user-guide/README.md)
- [Dev Guide](docs/dev-guide/README.md)
- [Project Contract](docs/dev-guide/project-contract.md)
- [Office Federation Contract](docs/dev-guide/office-federation-contract.md)
- [Architecture](docs/dev-guide/architecture.md)

## UI Engineering Rule

Zebflow UI work is expected to use **Zeb React** and **Zeb Tailwind**. If a normal Zeb/RWE UI flow behaves strangely, that should be treated as a **foundational RWE issue** and fixed at the root instead of being bypassed with page-local hacks or DOM workarounds.

## License

MIT — see [LICENSE](LICENSE).
