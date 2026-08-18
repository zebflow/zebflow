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

## Assistant Knowledge

A project carries three assistant documents. They are not one thing, and the
difference decides where each belongs and who may write it.

| Document | Declares | Written by | Authority | Scope |
| --- | --- | --- | --- | --- |
| `AGENTS.md` | the project's agents, their tools, their constraints | a human, or the assistant **when asked** | rules the assistant follows | the project |
| `SOUL.md` | assistant personality, style, and tone | a human | rules the assistant follows | the project |
| `MEMORY.md` | what an assistant noticed while working | the assistant, freely | **none — background only** | **one user** |

### Memory has no authority until a human promotes it

An assistant may notice anything. Nothing it notices governs the project until a
human says so, and that promotion is an edit to `AGENTS.md`: a change with a
diff, a reviewer, and a history.

This has to be real rather than declared. If memory is loaded as instruction,
calling it "background" is laundering, and unconsented text still steers
behaviour. Memory informs; only the authored documents instruct.

### Memory belongs to a user, not to a project

Project-shared memory written without consent is worse than personal memory,
because one person's session silently shapes every colleague's assistant with no
attribution and no review. Assistant memory is therefore scoped to the person
who produced it.

`AGENTS.md` and `SOUL.md` are the opposite: they are the project's declared
rules, shared deliberately, and every member should see the same ones.

### What this requires

Two placements follow from the above and neither is what exists today.

`AGENTS.md` and `SOUL.md` are authored declarations, so they belong under `repo/`
with the project's other source, where git records how a project's rules
changed. They currently sit under `data/runtime/agent_docs/`, which is the area
defined as machine-produced and safe to discard.

`MEMORY.md` is correctly derived and correctly under `data/`, but it is keyed by
the project owner rather than by the person working. A project has members
beyond its owner, so the current path cannot express per-user memory at all.

Memory also accumulates without bound. A file gives no eviction and no
retrieval, which are the properties memory actually needs. Whether it stays a
document or becomes a record is open, and is the same question
[`InvocationRecord`](./kinds/invocation-record/README.md) raises about bounded,
disposable history.

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
