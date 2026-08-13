# Project Contract

This contract is for people and tools that build and run Zebflow projects.

## Main Promise

A project can use documented Zebflow features without knowing the internal Rust
code or worker layout.

## Project Owned Work

A project owns its declared:

- source and project settings
- pipelines, functions, scripts, pages, components, and styles
- database schemas and initial data
- files and generated outputs
- project credentials and access rules
- active runtime copies
- Hub package settings
- invocation history when retention is enabled

## Stable Public Areas

Project work may rely on documented:

- project manifests and lock files
- pipeline graph and DSL meaning
- node kinds, settings, pins, input, output, examples, and errors
- expression roots and run context
- RWE, Zeb React, and Zeb Tailwind behavior
- database query and mutation nodes
- FileRef and ZebFS behavior
- credential kinds and outbound request rules
- map, realtime, scheduler, and trigger behavior
- Hub review, add, publish, and project transfer
- HTTP and MCP project interfaces
- stable error codes

## Same Meaning on Every Surface

A feature must keep the same meaning in the visual editor, DSL, HTTP API, MCP,
and generated UI. The controls may look different, but validation, security,
and runtime behavior must agree.

Native, composite, and WASM nodes use the same public node contract.

## Data Movement

Small JSON values may move directly between nodes. Files and large values use a
stable reference or another bounded runtime handle. This internal change must
not surprise downstream code that follows the documented value rules.

Temporary paths are not durable project files. Project code must not require an
undocumented temporary path after the run ends.

## Import and Update Safety

Before adding a package or updating a runtime, Zebflow must report known file
conflicts, executable code, credentials, external URLs, database changes,
public routes, schedules, and format problems. Failure must leave the previous
valid project state available.

Project tools must learn from committed contracts and generated definitions,
not from old conversation history.
