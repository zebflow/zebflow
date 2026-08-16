# Zebflow

**[zebflow.com](https://zebflow.com)** · [Docs](docs/README.md) · [Usage](docs/usage/README.md) · [Developer](docs/developer/README.md)

> One runtime for building and running full-stack apps.

Zebflow helps you build web apps, APIs, workflows, maps, databases, realtime features, and AI tools from one running system.

It is for people who want to turn ideas into working software without managing many separate stacks. You can write pages, create backend logic, store data, publish files, build maps, run agents, and share reusable work from the same project.

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-zebflow%2Fzebflow-informational.svg)](https://hub.docker.com/r/zebflow/zebflow)

## Why Zebflow Exists

Modern apps often need too many separate parts:

- frontend build tools
- backend APIs
- databases
- file storage
- workflow runners
- map servers
- realtime servers
- credentials
- logs and traces
- deployment scripts
- AI tools

Zebflow brings these parts into one project runtime. You can start small on your laptop, then move the same project toward a server, office, worker, or cluster setup when needed.

## What You Can Build

- Internal tools and dashboards
- Data apps
- GIS and map apps
- Research demos
- AI-assisted workflows
- Realtime apps and games
- Webhook and API automation
- Project websites and admin panels

## What Is Inside

### Project Studio

The browser UI where you edit pages, pipelines, files, databases, Hub packages, and project settings.

### Reactive Web Templates

Write TSX pages directly in Zebflow. You do not need a separate frontend app or npm build step for project UI.

### Pipelines

Create backend logic as graph workflows: webhooks, APIs, schedules, functions, file processing, database queries, AI calls, and map publishing.

### Sekejap DB

Every project gets an embedded multi-model database. You can store normal records, graph links, spatial data, vectors, and text search data without installing an external database.

### Files and Storage

Upload, save, publish, and process project files through Zebflow's file system.

### Mapserver and GIS

Publish GeoJSON, GeoParquet, map tiles, map layers, point queries, statistics, and function-backed live map data.

### MCP and AI Tools

Expose project actions to AI agents through MCP. Agents can inspect, edit, run, and debug project work through project-scoped tools.

### Hub

Share and add reusable packages: templates, pipelines, libraries, components, examples, and node bundles.

## Install

Choose the path that fits your machine.

### npm

Many users already have Node.js installed. This is the shortest path when you want a familiar command:

```bash
npm install -g zebflow
zebflow
```

Then open:

```text
http://localhost:10610/login
```

### pip

Python users can install Zebflow with pip:

```bash
pip install zebflow
zebflow
```

Then open:

```text
http://localhost:10610/login
```

Setting `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD` is optional. When it is omitted on
the first run, Zebflow generates a strong password and prints the path to its
private file under the platform data directory. Existing superadmin passwords
are never replaced during startup.

### Docker

Docker is useful for servers and repeatable deployments:

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
docker run --name zebflow \
  -p 10610:10610 \
  -v zebflow-data:/var/lib/zebflow/data \
  -e ZEBFLOW_PLATFORM_DEFAULT_PASSWORD \
  zebflow/zebflow:latest
```

Then open:

```text
http://localhost:10610/login
```

### Source Build

Rust users can run from source:

```bash
export ZEBFLOW_PLATFORM_DEFAULT_PASSWORD="$(openssl rand -base64 32)"
cargo run --bin zebflow
```

## Quick Start

1. Run Zebflow.
2. Open Project Studio.
3. Create a project.
4. Add a page or pipeline.
5. Run it.
6. Share reusable work through Hub when it is ready.

## Examples

Examples should be runnable projects. They are used for learning, testing, demos, and Hub publishing.

Curated examples will be added only after their behavior and package format are
verified against the stable contracts.

## Runtime Modes

One binary can run in different roles:

```bash
zebflow
zebflow controller
zebflow office
```

- `zebflow` starts a standalone runtime.
- `zebflow controller` starts the control-plane role.
- `zebflow office` starts the execution/worker role.

Most users should start with plain `zebflow`.

## Documentation

- [Docs Home](docs/README.md)
- [Usage Guide](docs/usage/README.md)
- [Developer Guide](docs/developer/README.md)
- [Platform Contract](docs/contracts/platform.md)
- [Project Contract](docs/contracts/project.md)
- [Versioning Contract](docs/contracts/versioning.md)

## Design Rule

Zebflow UI work should use Zeb React and Zeb Tailwind. If a normal Zeb/RWE UI flow behaves strangely, fix the root RWE behavior instead of adding page-local workarounds.

## License

MIT.
