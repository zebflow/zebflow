# What a running node can reach

Status: **survey**. Every claim below was checked against the code as written on
2026-08-21. Where a previous document disagrees, this one was verified and that
one was not.

The three escapes this document named in §2 are now closed; §2 records how, and
what is left. The first of the two declarations in §3 is now enforced; §3
records where, and what it does not cover.

Zebflow's install boundary is now strong: a package is reviewed before it lands,
refused for file types no project accepts, refused at publish if no instance
would install it, and disclosed to a user before they consent.

Almost none of that governs what a node can reach **once it is running**.

## 0. The mismatch this exists to name

Capabilities are now *derived* exactly — `Network`, `Filesystem`, `Database`,
`Credential`, `Process`, computed from what a package composes rather than from
what it claims. That derivation is **disclosure only**.

The review states that `n.script` can execute processes and that
`n.http.request` can reach the network and read a credential. Nothing prevents
either from doing more than was derived, because nothing enforces the ceiling.

A capability is currently a claim *about* a package. Confinement is what would
make it a limit *on* one. `spec.hosts` is the one declaration that has become a
limit, and it is an author's statement rather than a derived capability — §3
records what it covers.

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

## 3. Declarations, and what honours them

**`spec.hosts`** — a node bundle declares the hosts it intends to contact. The
contract validates the list (`contracts/kinds/node.rs:217`): count, duplicates,
and shape. It is now also enforced at run time, for bundle-provided nodes.

`BundleEgress` in `pipeline/security.rs` is that allowlist. It is attached to
the engine that runs a bundle's function pipeline — in `composite_host.rs`,
where the manifest is already resolved by kind, and in the lifecycle-hook runner
in `web/mod.rs` — so it governs the whole inner subtree rather than the one node
a project's graph names. That placement is the point: a bundle declaring
`["api.openai.com"]` and composing `n.http.request` would otherwise reach
anywhere, which was the entire hole.

A violation fails the node, naming the host refused and the bundle that refused
it:

```text
n.http.request outbound host 'elsewhere.invalid' is not declared by node bundle
'undeclared' (spec.hosts: declared.invalid)
```

Nothing new happens to the run: the inner node fails, and the composite reports
it on the `error` pin the engine already routes failures through. The request is
never made, and it is never dropped silently either.

Four consequences follow from where the check sits, each a stated limit rather
than an omission:

- **A composite inside a composite answers to both declarations.** The policy
  descends with the dispatch instead of being replaced by it, so a bundle cannot
  widen its own list by composing a more permissive one.
- **A network node whose destination never reaches a guard as a URL is refused,
  not allowed.** `n.ai.agent`, `n.pg.query`, `n.table.query`,
  `n.ws.client.send` and `n.trigger.ws.client` reach hosts that come from a
  credential or a project connection, which the egress guard never sees. Inside
  any bundle they fail with `FW_EGRESS_UNCHECKED_NODE`. The set is derived from
  `native_node_capabilities()` — the same table the package review reads — so a
  network node added later is refused until it is given a guard, rather than
  silently becoming the way out.
- **An empty or absent list restricts no host, and buys nothing else.**
  `#[serde(default)]` makes the two identical on the wire, so reading empty as
  deny-all would break every bundle published before enforcement existed. That
  much is deliberate and temporary. What does *not* follow from it is a weaker
  bundle: the refusal above runs for every bundle-provided node whether its
  bundle declared a host or not, so declaring nothing is not a way to obtain
  `n.pg.query`. Only the host allowlist is affected by an empty list, and an
  empty allowlist is the one thing an author gains nothing by choosing.
- **`n.script` answers to the sandbox actually in force.** The Deno sandbox
  denies `fetch` as shipped, so a script reaches nothing a host guard would need
  to read, and both curated bundles compose one. Where an operator has granted
  the sandbox network access — `dangerZone.allowNet`, or any
  `allowList.externalFetchHosts` entry — a script becomes egress the guard
  cannot read, and is refused on the same grounds as the rest. The guard reads
  the merged platform and project patches through `LanguageEngine::grants_network`
  rather than the shipped default, so widening the sandbox narrows what a bundle
  may compose in the same motion. No caller in the repo sets either network
  field on those patches today — `for_project` sets a fetch root and nothing
  else — so the check is inert until an operator-configuration path exists to
  set one.

Scope is bundle-provided nodes only. A project's own pipeline calling
`n.http.request` carries no policy and is not restricted: the threat model is
third-party code the user installed, not the user's own work.

Still open in the same area, named rather than fixed:

- `n.function.call` lets a bundle start a pipeline the project wrote, and that
  pipeline runs unrestricted. What it reaches is the user's own code, but the
  bundle chose the moment.
- The check is on the host, not on which credential travels to it. The second
  violation the bundle contract names — a credential value reaching a host other
  than the one that credential belongs to — is still unimplemented.

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
directory escapes left, one declaration a package cannot exceed, and one
declaration nothing honours.

`spec.hosts` was the item in §3 that needed designing rather than fixing: where
an outbound connection is checked, and what a violation does to a run in flight.
Both are answered now — at the dispatch that already knows which bundle it is
inside, and by failing the node through the error pins the engine already has.

What is left is narrower than what it replaced. A ceiling that holds for the
node kinds Zebflow can read a destination from, refuses the ones it cannot
whoever provided them, and says plainly that a bundle which declares nothing is
bounded by nothing *in which hosts it may name* — and by everything else.
