# Platform Contract

This contract is for people who change Zebflow itself.

## Main Promise

Zebflow connects project source, pipelines, web pages, data, files, maps,
credentials, realtime work, packages, and runtime execution in one system.

The Rust modules and internal methods may change. Existing valid project work
must keep the same meaning unless a major release provides a safe migration.

## Required Rules

The platform must:

- keep each project's source, data, files, credentials, and runtime state in the
  correct project scope
- keep durable project state outside short lived process memory
- validate known formats before activation or execution
- use node definitions for validation, editor forms, DSL, MCP, tools, and help
- give native, composite, and WASM nodes one public node model
- preserve FileRef meaning across nodes and storage backends
- limit payload copies, fan out, traces, and nested function results
- give logs and runtime files a clear owner and retention rule
- keep liveness available when project work is busy
- reject an unsafe update before it changes durable data
- support project inspection, transfer, backup, and recovery through declared
  project files and data

## Internal Details

Projects must not depend on:

- Rust file names or private traits
- thread and task layout
- cache and temporary file layout
- one database pool design
- one storage backend
- one worker placement method
- compiler or renderer helper functions

These details can change when tests prove that public behavior remains the same.

## Expensive Work

Work with large data or long run time needs a size limit, time limit, load
control, or background job boundary. A single project run must not block
health, project management, or unrelated projects without a clear limit.
