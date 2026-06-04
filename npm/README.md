# zebflow

Reactive web automation platform. One binary. No build step.

Write a TSX page, register a pipeline, it's live.

## Install

```bash
npm install -g zebflow
```

## Usage

```bash
zebflow              # start server on port 10610
zebflow --help       # show options
```

## What is Zebflow?

- **Templates** — TSX with SSR, no build step
- **Pipelines** — visual workflow DSL (40+ built-in nodes)
- **MCP** — AI agent integration out of the box
- **Single binary** — Rust, ~110MB, ~150MB RAM idle

## Other install methods

```bash
# curl
curl -fsSL https://raw.githubusercontent.com/zebflow/zebflow/main/install.sh | sh

# pip
pip install zebflow

# cargo
cargo binstall zebflow

# docker
docker run -p 10610:10610 zebflow/zebflow
```

## Links

- Website: https://zebflow.com
- GitHub: https://github.com/zebflow/zebflow
- Docs: https://zebflow.com/docs
