# What a running node can reach

Status: **survey**. Every claim below was checked against the code as written on
2026-08-21. Where a previous document disagrees, this one was verified and that
one was not.

The three escapes this document named in §2 are now closed; §2 records how, and
what is left.

Zebflow's install boundary is now strong: a package is reviewed before it lands,
refused for file types no project accepts, refused at publish if no instance
would install it, and disclosed to a user before they consent.

None of that governs what a node can reach **once it is running**.

## 0. The mismatch this exists to name

Capabilities are now *derived* exactly — `Network`, `Filesystem`, `Database`,
`Credential`, `Process`, computed from what a package composes rather than from
what it claims. That derivation is **disclosure only**.

The review states that `n.script` can execute processes and that
`n.http.request` can reach the network and read a credential. Nothing prevents
either from doing more than was derived, because nothing enforces the ceiling.

A capability is currently a claim *about* a package. Confinement is what would
make it a limit *on* one.

## 1. Boundaries that hold

**WASM is capability-zero by construction.** `wasm_host.rs` calls
`Instance::new(&mut store, &module, &[])` — an empty import array — and
`Cargo.toml` builds `wasmtime` with `default-features = false` and no WASI. A
module gets linear memory and arithmetic and no host function at all. A module
that declares an import fails to instantiate.

**The Deno sandbox denies everything by default.** `DenoSandboxDangerZone`
derives `Default`, so `allow_net`, `allow_run`, `allow_import`,
`allow_dynamic_code` are all false and the path lists are empty. Widening comes
only from `platform_patch` and `project_patch` — operator configuration.

**A package cannot widen its own sandbox.** `extract_run_patch` reads
`metadata.languageRunPatch`, and `execution_metadata` builds `metadata` from a
fixed key set (`owner`, `project`, `pipeline`, `request_id`, `route`, `trigger`,
`nodes`, `placeholder`). The only mutations anywhere insert `reduce_acc` and
`route`, both engine literals. There is no path from a package's JSON into that
key. The one in-repo writer of `languageRunPatch` is the expression resolver,
which *narrows* to `capabilities: []` and `maxOps: 500`.

**Filesystem nodes go through ZebFS**, which is project-scoped, rather than raw
paths. `fs_pdf_convert` sanitises its output path with `sanitize_rel_path`.

**Platform shell and git run in the project.**
`platform/shell/executor.rs` and `adapters/file/mod.rs:56` both
`current_dir(&layout.repo_dir)`.

**`n.ai.agent`'s shell tools run in the project.** `Node::shell_work_dir`
resolves `layout.repo_dir` through the same `ensure_project_layout` the
filesystem nodes use. A run with no platform service, or with no owner/project,
refuses the tool with that reason rather than running it somewhere else.

**A shell tool's root is a boundary.** `shell_tools::resolve_in_work_dir`
resolves `..` lexically, then re-checks against the real filesystem to catch a
symbolic link, and refuses an argument landing outside the root with an error
naming both the path and the root. Nothing is clamped back inside.

**The script sandbox fetches inside one project.**
`PlatformService::project_sandbox` builds a `DenoSandboxEngine::for_project`
rooted at `layout.files_dir`, entering through the project patch layer.

## 2. Boundaries that were escapes

All three were the same shape: the server process's own directory standing in
for a project's. Each is closed, and none was closed by clamping.

| Where | Was | Is |
| --- | --- | --- |
| `pipeline/nodes/basic/agent.rs` | `work_dir = std::env::current_dir()` | the owning project's `repo_dir`, or a refusal naming why there is none |
| `automaton/infra/repl.rs` | same | `shell_tool_root()` — the operator's directory or `ZEBTUNE_WORK_DIR`, logged at start |
| `language/engines/deno_sandbox/config.rs` | `local_fetch_root: ".".into()` | empty, meaning no root; the project's `files_dir` arrives via the project patch |

The sandbox default is now fail-closed rather than fail-to-the-server: an engine
built for no project refuses a local fetch by name, so a construction site that
forgets to scope one closes that project's scripts instead of opening the
server's directory. `op_read_local_file` marks a refusal `local fetch denied:`
and the fetch wrapper rejects on that marker, so a refused path can no longer
be read as a 404.

**Still open, and named rather than fixed:**

- **The scheduler's engine has no single project.** `web/mod.rs` builds one
  `BasicPipelineEngine` for the scheduler, the KV subscriber, and the WS client
  manager, all of which serve every project. Its sandbox has no fetch root, so a
  scheduled `n.script` cannot local-fetch at all. That is the safe answer, not
  the right one; the right one is a per-run engine.
- **The REPL has no project.** `zebtune` is a standalone binary with no
  Zebflow project on disk; its `owner`/`project` are automaton-context
  placeholders. Its root is the operator's own directory, which is stated in the
  log rather than assumed.

## 3. Declared and not enforced

**`spec.hosts`** — a node bundle declares the hosts it intends to contact. The
contract validates the list (`contracts/kinds/node.rs:217`): count, duplicates,
and shape. Nothing checks an outbound connection against it at runtime. The
phrase used when it was added was "declared now, enforced later", and that is
still exactly true.

Enforcement is newly practical: `Network` is now derived rather than trusted, so
the set of nodes that can egress is known without asking the package.

**`script_available`** — every node declares whether it may be called from
inside an `n.script` sandbox. Nothing in `src/language/` dispatches from the
sandbox back into node handlers, so the flag currently describes a bridge that
does not exist. Not a hole; an unimplemented feature whose declaration is
already in place.

**Derived capabilities** — reported to a user, never checked against what a node
does.

## 4. What follows

The install boundary answers "should this be here". The runtime boundary answers
"what may it do now that it is". The first is well defended; the second has no
directory escapes left and two declarations nothing honours.

What remains in §3 is a different kind of work from what §2 was. The escapes
were each a one-line divergence from a pattern the codebase already used
correctly, closable without designing anything. Enforcing `spec.hosts` is not:
it needs a decision about where an outbound connection is checked and what a
violation does to a run in flight.
