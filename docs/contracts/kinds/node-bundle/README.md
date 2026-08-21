# NodeBundle

Status: **Frozen**

`NodeBundle` describes one installable package that provides one or more node
kinds and the artifacts they run. It owns everything `NodeDefinition` refuses to
own: release identity, artifact inventory, integrity, credentials, composite
functions, WASM modules, trigger roles, lifecycle hooks, and installation.

A bundle is the only install source. One node and many nodes use the same
`definition.json`. A one-node bundle is simply `spec.nodes.length == 1`.

## Identity

| Item | Value |
| --- | --- |
| API version | `zebflow.com/v1` |
| Kind | `NodeBundle` |
| Representation | Strict UTF-8 JSON envelope |
| Owner | Platform node packaging |
| Durable source | `data/nodes/{package-slug}/definition.json` |
| Platform bundles | `src/pipeline/nodes/bundled/{package-slug}/`, consumed at build time |
| Rust model | `src/platform/model.rs` |
| Contract and validator | `src/contracts/kinds/node.rs` |
| Runtime registry | `src/platform/services/node_registry.rs` |
| Lock integration | `src/platform/services/dependency_lock.rs` |
| Golden fixtures | `tests/fixtures/contracts/node-bundle/` |

`metadata.name` must equal `spec.package`. `metadata.version` must equal
`spec.version`. A bundle release version is not a contract format version.

## Package Layout

```text
{package-slug}/
  definition.json
  icon.svg
  icons/*.svg
  functions/*.zf.json
  wasm/*.wasm
```

Every artifact is a regular file inside the package root. Symlinks, absolute
paths, parent traversal, and files outside the root are rejected.

This layout is identical wherever a bundle appears: authored in
`src/pipeline/nodes/bundled/` for platform bundles, published to the Hub, and
materialized into `data/nodes/` on install. One layout, three consumers.

### Why installed bundles live under `data/`

A project has three areas, and they are categories of ownership rather than
storage buckets:

| | |
| --- | --- |
| `repo/` | what the human declares — authored, versioned, git-tracked |
| `data/` | what the machine derives and keeps — materialized, rebuildable |
| `files/` | what the application stores for its users |

An installed bundle is not declared, it is *materialized from* a declaration.
The declaration is `zeb.lock`, which is authored and stays in `repo/`. The bytes
are the machine's output, so they belong in `data/nodes/`. This mirrors the
split that already exists between `repo/pipelines/` and the activated snapshots
under `data/runtime/pipelines/`.

A bundle's `entry` path in `zeb.lock` is unchanged by this: it is still
`nodes/{slug}/definition.json`, now resolved against the project's `data/` root.

## Canonical Shape

```json
{
  "apiVersion": "zebflow.com/v1",
  "kind": "NodeBundle",
  "metadata": {
    "name": "ml",
    "version": "1.0.0"
  },
  "spec": {
    "package": "ml",
    "version": "1.0.0",
    "title": "Machine Learning",
    "description": "Dataset loading, gradient boosting, and job triggers.",
    "icon": "icon.svg",
    "credentials": [
      {
        "kind": "ml_api",
        "title": "ML API Key",
        "description": "Credential used to reach the dataset service.",
        "config_key": "credential_id"
      }
    ],
    "functions": {
      "load": "functions/load.zf.json",
      "on_job_done": "functions/on-job-done.zf.json"
    },
    "modules": {
      "core": { "path": "wasm/ml.wasm", "abi": "zebflow-wasm-json-v1" }
    },
    "nodes": [
      {
        "kind": "n.x.ml.dataset.load",
        "title": "Load Dataset",
        "description": "Fetch and parse a training dataset.",
        "icon": "",
        "ui_category": "ml",
        "ui_category_label": "Machine Learning",
        "uses_credentials": ["ml_api"],
        "run": { "function": "load" },
        "definition": {
          "input_pins": ["in"],
          "output_pins": ["out"],
          "config_schema": {
            "type": "object",
            "properties": { "credential_id": { "type": "string" } }
          },
          "input_schema": {},
          "output_schema": {},
          "examples": [],
          "failure_semantics": [],
          "fields": [],
          "layout": [],
          "dsl_flags": []
        }
      },
      {
        "kind": "n.x.ml.gb.train",
        "title": "Train Gradient Boosting",
        "description": "Train a model from the incoming dataset.",
        "icon": "icons/train.svg",
        "ui_category": "ml",
        "ui_category_label": "Machine Learning",
        "uses_credentials": [],
        "run": { "module": "core", "export": "gb_train" },
        "definition": {
          "input_pins": ["in"],
          "output_pins": ["out"],
          "config_schema": { "type": "object" },
          "input_schema": {},
          "output_schema": {},
          "examples": [],
          "failure_semantics": [],
          "fields": [],
          "layout": [],
          "dsl_flags": []
        }
      },
      {
        "kind": "n.x.ml.job.done",
        "title": "On Training Finished",
        "description": "Start a pipeline when a training job reports completion.",
        "icon": "",
        "ui_category": "ml",
        "ui_category_label": "Machine Learning",
        "uses_credentials": [],
        "trigger": {
          "type": "webhook",
          "path_template": "/ml/done/{{ job_key }}"
        },
        "run": { "function": "on_job_done" },
        "lifecycle": {
          "on_activate": { "function": "load" },
          "on_deactivate": { "module": "core", "export": "cleanup" }
        },
        "definition": {
          "input_pins": [],
          "output_pins": ["out"],
          "config_schema": {
            "type": "object",
            "properties": { "job_key": { "type": "string" } }
          },
          "input_schema": {},
          "output_schema": {},
          "examples": [],
          "failure_semantics": [],
          "fields": [],
          "layout": [],
          "dsl_flags": []
        }
      }
    ]
  }
}
```

## Size And Count Limits

| Item | Limit |
| --- | --- |
| `definition.json` serialized size | 4 MiB |
| Nodes in one bundle | 256 |
| Functions in one bundle | 256 |
| Modules in one bundle | 64 |
| Credential definitions in one bundle | 32 |
| Files in one package directory | 4096 |
| One WASM module | 32 MiB |
| One normalized `NodeDefinition` | 512 KiB |

The node limit matches `MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS` so a valid
bundle can always be recorded in `zeb.lock`.

## Field Rules

### Release Identity

`spec.package` is a lowercase slug of ASCII letters, digits, and internal
hyphens. `spec.version` is a numeric `major.minor.patch` release. `title` and
`description` are required and non-empty.

Package names are project-scoped. Two bundles installed into the same project
may not share a slug.

### Node Kind Ownership

The namespace a bundle may use depends on **where it is consumed**, not on
anything the document declares. One authoring contract, two consumption paths.

| Scope | Consumed at | Namespace | Guaranteed present |
| --- | --- | --- | --- |
| `Platform` | build time, shipped in the binary | `n.*` | yes |
| `Project` | install time, into one project | `n.x.{package_token}.*` | no |

Zebflow curates `n.*` and guarantees uniqueness there, which is why a platform
bundle needs no package scoping. Everyone else gets `n.x.{package_token}.*`,
where `package_token` is `spec.package` with every hyphen replaced by an
underscore, because kind segments allow underscores but not hyphens.

```text
platform bundle             → n.telegram.send
package "ml"                → n.x.ml.gb.train
package "openai-embedding"  → n.x.openai_embedding.embed
```

For a project-scope bundle, kind ownership is structural: the kind names the
package that provides it, so two installed bundles can never claim the same
kind, and uninstall can prove which kinds a bundle owns.

This rule is enforced by `validate_bundle_namespace(spec, scope)` rather than by
`NodeBundleContract::validate`, because the same bytes are legal in one scope
and illegal in the other. A curated bundle may not use `n.x.`, and an installed
bundle may not claim a curated name.

The remaining segments are free. `NodeDefinition` still governs kind syntax:
lowercase dot-separated segments of ASCII letters, digits, and underscores.

Neither implementation nor availability is encoded in the kind. A node may move
between composite and WASM without changing its kind, and whether a node is
currently installed is answered by the registry and `zeb.lock`, not by its name.
Promoting a third-party package into the curated namespace is therefore a
rename, and must be a deliberate versioned event rather than a quiet blessing.

### Run Binding

`run` declares where a node's code lives. It has exactly one form:

| Form | Meaning |
| --- | --- |
| `{ "function": "name" }` | Composite. `name` is a key in `spec.functions`. |
| `{ "module": "name", "export": "symbol" }` | WASM. `name` is a key in `spec.modules`. |

`export` is required whenever `module` is present. There is no default export
symbol, because a default would make two WASM nodes in one bundle resolve to the
same entry point.

An action node must declare `run`. A trigger node may omit it, which means the
trigger emits its event payload unchanged.

Implementation type is derived from this binding. There is no `source` field,
so nothing can contradict the artifact a node actually points at.

| Node declares | Implementation |
| --- | --- |
| `run.function` | composite |
| `run.module` and `run.export` | wasm |
| no `run`, with `trigger` | declarative |

### Functions And Modules

`functions` maps a name to a package-relative `.zf.json` path. Each file must
exist and must be a valid frozen `Pipeline` contract.

`modules` maps a name to `{ path, abi }`. Each module file must exist, stay
within the size limit, and parse under the host. `abi` belongs to the module
because a bundle may carry artifacts compiled at different times.

Only `zebflow-wasm-json-v1` is accepted under `zebflow.com/v1`. An unknown ABI
fails validation and execution.

Declared names that no node references are rejected. A bundle states exactly the
artifacts it uses.

### Trigger

`trigger` declares a role, not an implementation. `type` is one of `webhook`,
`ws`, `ws_client`, or `cron`. A `path_template` may reference configuration keys
with `{{ key }}`, and every referenced key must exist in the node's
`config_schema.properties`.

A trigger node has no input pin. Its inbound handler is `run`, which is why a
WASM trigger is expressible.

### Lifecycle

`lifecycle.on_activate` and `lifecycle.on_deactivate` are run bindings, using the
same two forms as `run`. Hooks are available to composite and WASM nodes alike.

Lifecycle hooks are only valid on a node that declares `trigger`.

### Credentials

`spec.credentials` declares credential kinds the package needs. It never carries
credential values.

A node lists the kinds it uses in `uses_credentials`. Validation applies only to
the nodes that reference a credential:

- every referenced kind is declared in `spec.credentials`
- the credential `config_key` exists in that node's `config_schema.properties`
- the same key is documented by a field and a DSL flag on that node

A node that references no credential carries no credential obligation. This is
what allows one bundle to provide unrelated nodes.

Credential kinds are unique within a bundle and may not collide with a built-in
credential kind.

### Icons

`spec.icon` is the package icon. A node may override it with its own `icon`. A
declared icon path must exist inside the package.

Unknown fields are rejected in the envelope and every typed object.

## Node Interfaces In `repo/`

`zeb.lock` identifies a bundle and lets Zebflow verify it, but identifying is
not the same as obtaining. A bundle from a private repository or a local archive
can be named and verified and still be unavailable on another instance. That is
the failure every plugin ecosystem hits: a shared graph that references modules
the receiver cannot get, and no way to tell what they were.

So a project also carries the **interface** of every third-party node it uses:

```text
repo/
  zeb.lock                        which bundle, which digest, which source
  pipelines/blog.zf.json          references n.x.acme.thing
  nodes/
    n.x.acme.thing.json           a frozen NodeDefinition document

data/
  nodes/acme/                     the materialized bundle
```

`NodeDefinition` is implementation-neutral by design, which is exactly what lets
it travel without its implementation. With it present, a graph still states the
node's pins, fields, and configuration, so the editor can render it, activation
can validate config against it, and someone can reimplement the node against a
known contract instead of guessing.

Rules:

- Only `n.x.*` kinds get an interface. Curated `n.*` nodes are guaranteed by the
  platform and carry no portability risk.
- Only kinds a project's pipelines actually reference. `repo/` states this
  project's dependencies, not a mirror of everything installed.
- The bundle stays authoritative. The interface is a copy pinned when written,
  so comparing them detects a bundle changing its interface under a project,
  which is a breaking change and should be visible as a source diff.
- An interface is **not** removed when its bundle disappears. At that moment it
  is the only remaining description of the node.
- An interface **is** removed when the last pipeline reference goes.

`NodeRegistryService::sync_project_node_interfaces` owns this, and runs after a
pipeline is saved and after a bundle is installed.

## Integrity

`zeb.lock` records one `integrity` value per bundle: a SHA-256 over the package
directory's normalized file inventory, computed by
`directory_tree_sha256` in `src/infra/io/durable.rs`. The hash covers every
regular file under the package root, in sorted relative-path order, including
each file's relative path and length. Symlinks fail the hash.

Nothing under `data/` is authored by hand, so a digest that no longer matches
the lock means tampering or an interrupted write **whatever the bundle's
source**. Every mismatch fails closed.

The check runs in `record_discovered_node_bundles`, which every registry refresh
calls before publication, so a drifted bundle fails the refresh and the previous
registry stays active. The repair is to re-materialize from the lock, not to
trust what is on disk.

A newly discovered package that no lock entry already provides is still pinned,
which is how a restored project adopts bundles that arrived with it.

Pipeline activation re-checks the digest of every bundle a pipeline depends on
through `validate_pipeline_dependencies`, and hard-fails on any mismatch
regardless of source.

Integrity is not re-verified on every pipeline hit. Verification belongs to
discovery, registry publication, and activation.

## Installation

Installation spans package files, `zeb.lock`, and the runtime registry. It is
**recoverable, not atomic**, and this contract does not claim otherwise.

The implemented sequence in `src/platform/services/hub.rs` is:

1. Take the per-repository install lock, so two installs cannot interleave.
2. Resolve and validate every destination path, and read all file bytes into
   memory. Nothing is written yet, so an unsafe path fails before any change.
3. Validate prepared pipeline sources against the frozen `Pipeline` contract.
4. Snapshot the current dependency lock.
5. Write each file with an atomic per-file replacement.
6. Register any pipelines the bundle provides.
7. Refresh the node registry, which validates the bundle against this contract
   and publishes through one atomic pointer swap, then record Hub provenance in
   `zeb.lock`.

Any failure at steps 5 to 7 runs recovery: written files are removed or restored,
the previous dependency lock is rewritten, and directories the install created
are removed up to the bounded repository root. The previous registry stays active
because publication only happens on a fully validated refresh.

The honest limit: steps 5, 6, and 7 touch separate durable objects. A process
kill between them can leave files on disk that the lock and registry do not know
about. The next refresh re-validates the whole `data/nodes` tree and fails closed
on an invalid bundle, so a partial install is detected rather than executed, but
it is not silently repaired.

Reinstalling the same slug replaces the whole bundle. An update may not silently
change which bundle owns an existing kind; the kind-to-package binding makes
such a change a slug change.

Uninstall removes only the package directory and lock entry the selected kinds
belong to. Shared or unrelated artifacts are untouched.

## Concurrency And Failure

Project bundles are scanned into a complete temporary registry and published in
one atomic pointer swap only after every bundle validates and the dependency
lock update succeeds.

An invalid bundle, missing `definition.json`, duplicate kind, official-kind
collision, unreadable directory entry, or symlinked package directory fails the
refresh. The previous valid registry stays active. Zebflow does not skip an
invalid project package and publish a partial catalog.

Official embedded bundles fail at startup when malformed.

## Security

Package review must expose files, node kinds, credential kinds, external URLs,
database effects, file effects, public endpoints, schedules, initial data, large
files, and WASM modules before install. The UI may not hide them.
`src/platform/policy/package.rs` owns that inspection.

**One review, every source.** A Hub package and a local archive go through the
identical scan. A Hub package is not safer, only published.

### Three gates

| Gate | Question | Behavior |
| --- | --- | --- |
| Contract validator | Is this a well-formed bundle? | Refuses path traversal, symlinks, unsupported ABI, kind collisions, missing artifacts, invalid function pipelines |
| Policy violations | Is this bundle safe to run? | **Refuses. Never overridable, whoever approves.** |
| Policy warnings | Does the user accept these effects? | Reported; the user decides |

A warning asks whether an effect is acceptable. A violation says the package
will not be installed at all. The precedent already exists: a bundle with a
symlink cannot be installed no matter who approves it. Violations extend that
from malformed to unsafe.

`PackageSafetyReview::violations` and `is_installable()` exist now and install
checks them before considering any approval the caller supplies. Two detectors
populate them for a bundle: a declared node kind this build already provides,
and a function pipeline whose bytes the review cannot read. A third — a file
type no project accepts — is read against the receiving project's repository and
does not apply to a bundle, whose files materialize under `data/` and never
enter `repo/`. All three refuse what a package *is*; none refuses one for what
it does, because a non-overridable refusal has to be precise enough not to
produce false positives.

### Declared hosts

`spec.hosts` lists the external hosts a package may contact. An empty list
states that the package makes no external calls.

```json
"hosts": ["api.telegram.org", "*.example.com"]
```

Entries are lowercase host names with an optional leading `*.` wildcard label.
Schemes, ports, paths, credentials, and single-label names are rejected, because
a declaration that can be read two ways cannot be enforced.

Declaring is what turns "here are the URLs this package contacts" from a
judgement the reader has to make into a consistency check against what the
author stated. Static scanning alone is weak — a URL built at run time from
configuration is invisible, and a compiled WASM module is opaque — so the
declaration's real purpose is to be the allowlist a runtime egress boundary
enforces.

The first of the two violations this list exists for is enforced now:

1. contacting a host the package did not declare — **enforced at run time**
2. a credential value reaching a host other than the one that credential belongs
   to — not implemented

`BundleEgress` in `src/pipeline/security.rs` is the runtime allowlist. It is
attached where a bundle's function pipeline is dispatched, so a declaration
covers every node that pipeline runs and not only the bundle node a project's
graph names. A composite reached from inside another bundle answers to both
declarations, because the outer bundle is still the reason the connection is
being made. A violation fails the node, naming the host refused and the bundle
that refused it.

Four limits, stated rather than implied:

- A **non-empty** list is an allowlist. An empty or absent one restricts no
  host: `#[serde(default)]` makes the two identical on the wire, so reading
  empty as deny-all would break every bundle published before enforcement
  existed. The prose above — "an empty list states that the package makes no
  external calls" — remains what an author means by it, not what the runtime can
  prove they meant.
- A node that reaches the network through a destination the egress guard never
  sees as a URL — `n.ai.agent`, `n.pg.query`, `n.table.query`,
  `n.ws.client.send`, `n.trigger.ws.client` — is **refused** inside any bundle,
  rather than allowed through unchecked. A bundle needing one of those cannot be
  confined by a host list today, and that is true whether it declared hosts or
  not: an empty list weakens the host allowlist and nothing else, so an author
  gains nothing by staying silent.
- `n.script` is refused inside a bundle only where the Deno sandbox has been
  granted network access — `dangerZone.allowNet`, or any
  `allowList.externalFetchHosts` entry. As shipped the sandbox denies `fetch`,
  so a script reaches nothing a host guard would need to read.
- Enforcement covers bundle-provided nodes. A project's own pipelines are not
  restricted by any bundle's declaration.

A bundle contains public metadata and artifacts only. Credential values stay in
the credential service. Manifest fields never imply extra host capability for a
WASM module.

## Runtime Boundary

Runtime consumes the normalized registry. It does not reread `definition.json`
on a node call. Pipeline activation checks a node instance against the active
registry. A package change becomes visible only after a complete successful
registry publication.

## Compatibility

Patch releases may improve validation messages and registry performance without
changing accepted or emitted fields.

Minor releases may populate existing optional fields more completely. They may
not rename fields, change field meaning, add a default export symbol, loosen
kind ownership, or accept another WASM calling convention under `zebflow.com/v1`.

Adding or removing a serialized field, changing the run binding forms, changing
credential scoping, or changing integrity meaning requires a new contract API
version and an explicit converter. A v1 reader rejects unknown and newer formats.

Zebflow is pre-release. There is no compatibility reader for the earlier
`n.c.*` / `n.wasm.*` kind namespaces, the package-level `wasm` runtime block,
the `main` function pointer, the `trigger.on_message` handler pointer, or
package-wide implicit credentials. `definition.json` in the shape above is the
one canonical install source.

## Test Evidence

### Proven now

Contract tests in `src/contracts/kinds/node.rs`:

1. golden round-trip for the composite, WASM, and mixed bundles in
   `tests/fixtures/contracts/node-bundle/`
2. implementation type derived from the run binding, including the declarative
   trigger case
3. two WASM nodes in one bundle resolving distinct exports
4. credentials scoped to the nodes that declare them
5. wrong kind, future API version, unknown field, and oversized document
6. an authored `source` field rejected
7. metadata and spec identity mismatch, invalid slug, invalid release version
8. kind outside the package namespace, and duplicate node kind
9. `module` without `export`, mixed run forms, empty run, action without run
10. unresolvable function and module references
11. declared but unreferenced function or module
12. lifecycle on a non-trigger node
13. unsupported trigger type, input pins on a trigger, undeclared path template key
14. a WASM trigger handler accepted
15. unsafe artifact paths and unsupported WASM ABI
16. credential problems: undeclared kind, duplicate kind, absent config key

Registry, lock, and runtime tests in `src/platform/services/`:

17. a bundle claiming a foreign kind fails the refresh, previous registry active
18. a missing declared artifact fails the refresh, previous registry active
19. an invalid composite function fails the refresh, previous registry active
20. every official node declares each credential kind whose config key it exposes
21. uninstall removes only the owning bundle's files and lock entry
22. Hub digest drift fails closed and preserves the previous lock
23. project digest drift is re-locked to the observed digest
24. failed Hub install restores files and the dependency lock
25. successful Hub install writes exact files, lock entry, and registry entries
26. a real installed WASM bundle runs two nodes through distinct exports of one
    shared module under `zebflow-wasm-json-v1`
27. a WASM trigger runs its handler export, so role and implementation are
    genuinely independent at run time
28. interfaces are written for third-party kinds only, pruned when the last
    reference goes, and kept when their bundle disappears
29. a carried interface resolves while its bundle is absent and yields to the
    registry once it is installed
30. an interrupted install recovers or fails closed: a package that landed
    without a lock entry is adopted and pinned, a package without a manifest
    fails the refresh with the previous registry still active, and removing the
    partial package restores a clean refresh

Declared-host enforcement, in `src/pipeline/engines/composite_host.rs`,
`src/pipeline/engines/basic.rs`, and `src/pipeline/security.rs`:

31. an installed composite bundle's **inner** HTTP node is refused when it
    contacts a host the bundle did not declare, with the error naming both
32. the same inner node reaching a declared host is admitted by that check
33. a bundle declaring no hosts may reach any host
34. a bundle reached from inside another bundle satisfies both declarations, and
    the outer one refuses a host only the inner one declared
35. a network node whose destination cannot be host-checked is refused inside a
    bundle that declares hosts
36. the same node is refused inside a bundle that declares none, so silence buys
    no capability
37. a project's own request is unaffected by an installed bundle's declaration
38. every embedded bundle runs under its own declaration: no inner node is
    refused as uncheckable, and every host its functions name outright is one it
    declared
39. `n.script` runs inside a bundle under the shipped sandbox, and is refused
    once that sandbox is granted network access

### Live and browser evidence, 2026-08-18

Verified on an isolated instance, covering both curated and third-party paths:

Curated:

- 68 nodes with `n.telegram.*` and `n.ai.embedding` in the curated namespace,
  `source` derived from the manifest, `available` on every item, and no kind
  left in a pre-`n.x.` namespace
- native `n.ai.agent` and composite `n.ai.embedding` coexist without collision
- the DSL registers a pipeline using a curated composite
- activation runs the composite `on_activate` hook, which executed
  `register-webhook` and reached the Telegram API
- the webhook route from the trigger's `path_template` resolves and returns 200
- a curated composite executes its function pipeline
- the shared edit dialog opens `n.telegram.trigger` with its fields

Third-party:

- a bundle supplied as a file was reviewed and installed over
  `POST .../nodes/install/review` and `POST .../nodes/install`
- the same flow through the Hub page: choose file, review every category,
  install, both endpoints returning 200
- bundle bytes landed in `data/nodes/acme/`, and both `n.x.acme.*` kinds
  appeared as `community` with `source: wasm`
- a third-party WASM node executed, returning its own export
- saving a pipeline that references them materialised
  `repo/nodes/n.x.acme.train.json` and `n.x.acme.score.json`, and nothing for
  curated kinds
- uninstall removed the bundle while the interfaces survived, and the catalog
  then reported both kinds with real pins and `available: false`
- reinstalling flipped them back to `available: true`
- browser console errors: zero; failed application requests: zero

### Regressions this verification found

Five, none caught by the test suite, all from the same cause: a namespace prefix
used as a proxy for a fact the namespace stopped carrying.

1. the DSL could not resolve a curated composite kind
2. engine dispatch skipped curated composites, so they could not execute
3. lifecycle hooks never ran for curated bundle nodes
4. webhook routes were not extracted from curated composite triggers
5. saving a pipeline through the DSL did not refresh node interfaces

Resolution is now by existence throughout: the node catalog or the package
manifest answers whether a kind is bundle-provided, and a kind the project
describes but cannot run reports that its package is missing rather than that
the kind is unknown.

### Known gaps, deliberately left

- No detector populates `violations` from what a bundle *does*; the two that
  apply to a bundle read what it is. `spec.hosts` exists so the declaration is in
  place before behaviour detectors land.
- Interface sync runs on the two user-facing pipeline write paths and on
  install. Other writers, including cluster runtime sync and project transfer,
  do not trigger it. One canonical writer would remove that class of omission.
- The editor renders a described-but-unavailable node as unknown. The catalog
  reports it correctly, so this is presentation, not contract.
