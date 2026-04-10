# Zebflow Architecture

> Current architecture reference for the code in `src/`.
> Historical long-form notes were moved to `docs/ARCHITECTURE.obsolete.v1.md`.

## 1. Positioning

Zebflow is a tiny full-stack reactive web runtime.

It combines:

- pipeline-driven execution
- TSX pages rendered on the fly
- sandboxed scripting with a shared `Tool.*` stdlib
- built-in API, realtime, scheduler, and automation surfaces

The practical product shape is:

- build React-style TSX pages and Tailwind-ish UI quickly
- serve SSR pages, hydrated SPA behavior, realtime room state, APIs, and background jobs from one project
- run it as a lightweight local tool today
- keep the seams clear enough to evolve toward multi-user, multi-project Kubernetes deployment later

Current reality:

- standalone is still the default deployment shape
- a first controller/office slice now exists in code
- direct runtime ingress, dedicated runtime provisioning, and strong cluster security hardening are
  still incomplete

## 2. Runtime Shape

`src/lib.rs` splits the product into six subsystems:

| Subsystem | Role |
|---|---|
| `pipeline` | graph execution, node dispatch, trigger handling |
| `language` | sandboxed JavaScript execution via Deno |
| `rwe` | TSX compile, SSR, and hydration |
| `automaton` | agent runtime and LLM-facing tool loop |
| `platform` | Axum web app, services, auth, studio UI, MCP, DSL shell |
| `infra` | shared runtime plumbing such as WebSocket rooms, scheduler, mem hub |

Current top-level composition:

```text
zebflow binary (standalone mode today)
  -> PlatformConfig
  -> PlatformService::from_config()
  -> platform::web::router()
       -> build platform frontend
       -> start scheduler
       -> start mem subscriber
       -> mount REST + pages + webhook + ws + MCP routes
```

The important boundary is:

- `platform` is the product shell and control surface
- `pipeline`, `language`, and `rwe` are the execution engines
- `infra` provides reusable runtime services shared by platform and nodes

## 3. What Runs Today

The main server entrypoint is `src/bin/zebflow.rs`.

Current runtime facts:

- default host/port is `127.0.0.1:10610`
- startup requires `ZEBFLOW_PLATFORM_DEFAULT_PASSWORD`
- `PlatformConfig::default()` uses owner `superadmin` and project `default`
- `zebflow` with no subcommand currently means standalone mode
- `zebflow master` and `zebflow controller` run the control-plane/server role
- `zebflow worker` and `zebflow office` run the execution-plane office role
- `dev.sh` is only a local convenience runner; it sets `admin123` and starts the same binary on `10610`

Boot flow in current code:

1. create the platform data root
2. open the SQLite catalog adapter
3. initialize the filesystem-backed project tree
4. bootstrap the default `superadmin/default` project if missing
5. reload active pipeline snapshots for every known project
6. build the Axum router and platform frontend
7. start `PipelineScheduler`
8. start `MemSubscriber`

Health endpoints are mounted directly in `src/platform/web/mod.rs`:

- `GET /health`
- `GET /ready`

## 4. Project Model

Each project is a git-backed local workspace under:

```text
{data_root}/users/{owner}/{project}/
```

Current layout from `src/platform/adapters/file/mod.rs`:

```text
repo/
  .git/
  pipelines/
  docs/
  zebflow.json
  zeb.lock

data/
  local.db
  runtime/
    pipelines/
    agent_docs/

files/
  public/
  private/
```

This split matters:

- `repo/` is the editable working tree and the git repository
- `data/runtime/pipelines/` stores activated pipeline snapshots
- `data/local.db` is the per-project SQLite runtime DB
- `files/` is the project-owned asset/file area

Platform-wide metadata is separate from project runtime data:

- global metadata catalog: `{data_root}/platform/catalog.db`
- per-project runtime DB: `{project}/data/local.db`

The catalog currently stores users, projects, credentials, DB connections, pipeline metadata, policies, MCP sessions, and pipeline invocation history.

## 5. Request and Interaction Surfaces

The current server exposes six main surface families:

| Surface | Purpose | Current route family |
|---|---|---|
| Platform pages | login, home, studio, settings, credentials, DB suite | `/login`, `/home`, `/projects/{owner}/{project}/...` |
| Project APIs | manage pipelines, templates, credentials, docs, DB connections, libraries, assets | `/api/projects/{owner}/{project}/...` |
| Webhook ingress | public/API entry into active pipelines | `/wh/{owner}/{project}/{*tail}` |
| WebSocket rooms | realtime room join and sync | `/ws/{owner}/{project}/rooms/{room_id}` |
| MCP | project-scoped remote operations | `/api/projects/{owner}/{project}/mcp` |
| Preview and assets | live preview and compiled/static assets | `/preview/...`, `/assets/...`, `/p/{owner}/{project}/assets/...` |

The platform studio itself is also rendered through RWE templates. It is not a separate frontend build product.

## 6. Pipeline Runtime Model

Pipelines are stored as `.zf.json` graphs under `repo/pipelines/`.

Important runtime rule:

- working tree source is editable draft state
- activated snapshots are the live production/runtime state

Activation in `ProjectService` copies the current pipeline JSON into:

```text
data/runtime/pipelines/{subpath}.{hash}.zf.json
```

`PipelineRuntimeService` then rebuilds the in-memory active registry from those snapshots, not from the mutable working tree.

That gives Zebflow two useful modes:

- draft editing in git-backed source
- stable live execution from activated artifacts

Current trigger extraction from active graphs:

- `n.trigger.webhook`
- `n.trigger.schedule`
- `n.trigger.ws`
- `n.trigger.weberror`
- `n.trigger.memsubscribe`

Execution engine facts from `src/pipeline/engines/basic.rs`:

- queue/BFS-style graph traversal
- per-node config expression resolution
- prior node outputs available through node metadata/state
- node traces recorded into invocation history
- support for trace redaction markers such as `__zf_private_redact*`

Background runtime services built around active pipelines:

- `PipelineScheduler` executes cron-triggered active pipelines
- `MemSubscriber` runs `n.trigger.memsubscribe` pipelines on channel messages
- `WsHub` manages realtime rooms used by WebSocket-triggered/state-sync pipelines

Canonical web response node today is:

- `n.web.response`

The older `n.web.render` framing is no longer the right mental model for current code.

## 7. Web Runtime: RWE

RWE is Zebflow's TSX compile/render system.

Current job:

- compile TSX-like project or platform templates
- inline project modules
- SSR through embedded V8
- produce client-side hydration code and script payloads

Current import rules from `src/rwe/core/compiler.rs`:

- `zeb`
- `zeb/*`
- `@/`

Other user-authored import roots are rejected.

Current render model from `src/rwe/core/render.rs` and `src/rwe/core/deno_worker.rs`:

- SSR runs inside embedded `deno_core`, not an external `deno` process
- a small worker pool is used (`RWE_WORKER_COUNT`, default `3`)
- workers are round-robin dispatched
- workers auto-respawn on death
- workers are periodically restarted after many renders to avoid memory accumulation
- SSR cache is in-memory, capacity `200`, default TTL `30s`
- a per-template circuit breaker opens after repeated failures

Current page response flow for project pages:

1. webhook route matches an active pipeline
2. webhook input is normalized into pipeline payload
3. template markup is loaded from project files for `n.web.response --template`
4. project RWE settings from `zebflow.json` are merged into the graph
5. `BasicPipelineEngine` executes the graph
6. `n.web.response` returns either JSON, redirect, text, or rendered HTML
7. compiled hydration scripts are externalized under the asset routes

Platform pages also use RWE, but they are precompiled at startup by `build_frontend()`.

## 8. Script Runtime and `Tool.*`

`language.deno_sandbox` is the current JavaScript runtime.

Current behavior from `src/language/engines/deno_sandbox/engine.rs`:

- source is compiled into `async function(input, n, ctx) { ... }`
- policy patches and loop guards are applied before execution
- runtime limits come from `DenoSandboxConfig`
- artifacts are serialized and can be re-run with per-run patches

The shared `Tool.*` stdlib lives in:

- `src/language/runtime/tool_init.js`

That same file is injected into:

- `n.script` execution
- RWE SSR contexts

So `Tool.time`, `Tool.arr`, `Tool.stat`, `Tool.geo`, and other helpers are shared across scripts and templates.

## 9. Users, RBAC, Credentials, and Runtime Services

`PlatformService` is the real composition root for product services.

Current services include:

- auth and authorization
- user profile settings and git identity resolution
- project membership and invite services
- project management
- credentials
- DB connections and DB runtime access
- assistant config
- active pipeline runtime
- pipeline hit counters
- MCP sessions
- library manifest registry
- `zeb.lock` and `zebflow.json` services

### 9a. Current User And RBAC Reality

The current code already has a real capability-based authorization core.

Today it already supports:

- platform users with `role`, `git_name`, and `git_email`
- project-scoped capabilities
- project policies
- project policy bindings
- authorization shared across REST, MCP, and assistant entrypoints
- superadmin bypass

Important current truth:

- current enforcement is capability-based, not route-hardcoded
- MCP already goes through the same project capability gate
- project policies are already the right low-level enforcement model

What is still incomplete in the product UX today:

- no real project members/invite UI yet
- no GitLab-style membership workflow in Studio yet
- MCP sessions are still effectively project-global, not yet truly user-bound

### 9b. `0.2.0` User Baseline

`0.2.0` should treat multi-user collaboration as a first-class architecture concern.

The baseline vocabulary should be:

- platform user
- user profile settings
- project member
- project invite
- role preset
- per-member MCP ceiling

The intended split is:

| Layer | Responsibility |
|---|---|
| User profile | personal git identity and future personal defaults |
| Membership | who can access a project |
| Role preset | product-facing access vocabulary such as `guest`, `reporter`, `developer`, `maintainer`, `owner` |
| Policy binding | low-level enforcement model used by authorization |
| MCP ceiling | maximum MCP capabilities that a member may delegate into a future user-bound MCP session |

The design rule is:

- membership is the UX model
- policies remain the enforcement model

That keeps the current authorization engine useful while giving the product a stable collaboration model.

### 9c. Git Identity Ownership

Git authorship should become user-bound by default.

The intended resolution order is:

1. current user profile
2. project git settings in `zebflow.json`
3. generated fallback identity

That is important because commits, settings saves, lock/unlock actions, and future MCP/assistant write operations should attribute changes to the acting user rather than one project-global author whenever possible.

### 9d. Role Presets

The product-facing role presets should follow a GitLab-like model:

- `guest`
- `reporter`
- `developer`
- `maintainer`
- `owner`

These presets should map into managed project policies.

Legacy names like `viewer` and `editor` may still exist as compatibility aliases during the transition, but the product UI and architecture vocabulary should move toward the GitLab-style names above.

Credential model today:

- project-scoped credentials live in the platform catalog DB
- runtime nodes resolve credentials explicitly through `CredentialService`
- credentials are typed by `kind`
- `secure_request` is now part of that credential model

DB model today:

- platform metadata uses SQLite catalog storage
- projects also get local SQLite runtime storage
- DB connections are first-class project records
- runtime DB operations are dispatched through `DbRuntimeService`

## 10. MCP, Assistant, and Agent Surfaces

Zebflow has multiple intelligence/control surfaces, but they should converge on the same business layer.

Current code already follows that direction:

- `PlatformOps` is the canonical implementation of project operations
- MCP delegates to `PlatformOps`
- assistant platform tools delegate to `PlatformOps`

Current surfaces:

| Surface | Current role |
|---|---|
| REST/API | browser and external control surface |
| MCP | project-scoped remote tool surface |
| Web assistant | browser-side assistant using platform tools |
| `n.ai.agent` | pipeline node that runs autonomous/direct agent behavior inside a pipeline |

`automaton/` is its own module and can be reasoned about separately from the web shell.

Current `n.ai.agent` modes:

- `direct`
- `strategic`

Both are exposed through one pipeline node in `src/pipeline/nodes/basic/agent.rs`.

## 11. Libraries and Install Reality

`zeb/*` libraries are a real part of the runtime architecture.

Current facts:

- embedded manifests are scanned into `LibraryService`
- a library can expose offline and online versions in its manifest
- project RWE library enablement is managed through `zebflow.json` and `zeb.lock`
- project and platform pages both consume the same library asset system

This is also how shared UI/editor/runtime pieces work today, including `zeb/codemirror`.

Important current limitation:

- the generic install/marketplace system is not done yet
- the studio install dialog currently has real backend support only for UI component installation
- `pipelines` and `scripts` tabs in the install dialog are still placeholders

So architecture documentation should not describe a generic package marketplace as current behavior.

## 12. Current Scalability Reality

Today Zebflow can run in two real shapes:

1. standalone monolith
2. split control/execution with one controller and one or more offices

What exists in code today:

- one Axum-based control-plane app per process
- one `PlatformService` composition root per process
- local filesystem project storage on each runtime host
- local SQLite catalog on each instance
- local per-project SQLite DB
- in-process scheduler, mem subscriber, and websocket room hub
- controller-side office registry and project placement records
- controller-proxied dispatch to a remote office for webhook and execute flows

What does **not** exist yet as finished architecture:

- direct public runtime ingress as the default production path
- dedicated runtime provisioning/orchestration
- mTLS-based controller/office transport
- shared external state backend
- cluster-wide autoscaling and multi-replica hot-state coordination

That still makes the current system useful:

- small enough for local development and single-host installs
- coherent enough to run on lightweight hardware
- already structured enough to start distributing projects across offices

## 13. Formal Office Federation Model

The office governance model is the normative control contract for Zebflow.

The full formal specification lives in:

- [OFFICE_FEDERATION_MODEL.md](/Users/mala0061/Dev/mecha.id/zebflow/docs/OFFICE_FEDERATION_MODEL.md)

The short version is:

- every Zebflow installation is an **office**
- `controller` is a governance role, not a different species of machine
- every office has:
  - a management domain
  - a local runtime domain
- every office has exactly one parent office
- an office is self-controlled iff its parent is itself
- a project is hosted by exactly one office at a time
- runtime availability must not depend on live controller reachability
- office binding changes only by explicit machine-level mutation, never by timeout

This yields the core architectural stance:

- the office is the embodiment of project runtime
- the controller is the mutation authority and observation lens
- platform auth is for mutation, not for steady-state runtime service
- application auth belongs to the project and remains independent

Current implementation note:

- the current clustered slice already supports office registration, project placement, materialized
  runtime sync, and direct office runtime handling
- Project Studio authoring is still controller-first today, which is a known mismatch with the
  target office-first model

## 14. Scale and Compute Direction

The scale philosophy must stay visible in architecture, but it has to be separated into:

- what exists today
- what the base must support next
- what will arrive later as extra execution backends

### 14a. Deployment Philosophy

Zebflow should remain:

- standalone by default
- lightweight enough for a single local install
- structured enough to scale into Kubernetes without rewriting the product model

The intended role model is:

| Role | Meaning |
|---|---|
| `zebflow` | standalone all-in-one install |
| `zebflow master` / `zebflow controller` | control plane |
| `zebflow worker` / `zebflow office` | execution-plane office |

Important design rules:

- even standalone should be understood as one logical controller plus one local office
- those two logical surfaces may share one process and one address in small deployments
- an office should host many project runtimes
- a project should later be able to have one or more runners
- office does not mean one office per project
- standalone mode remains a first-class deployment shape, not a special degraded mode

### 14b. Fresh `0.2.0` Rollout Model

The first clustered release should be treated as a fresh parallel system, not as an in-place mutation of an existing `0.1.x` install.

Recommended rollout shape:

1. keep the `0.1.x` production instance running unchanged
2. disable auto-update on that instance
3. bring up a fresh `0.2.0` cluster separately
4. clone projects from Git into the new cluster
5. rebuild repo-owned state from Git and `zebflow.json`
6. recreate or import environment-owned secrets separately
7. cut traffic only after validation

This split matters:

- repo-owned state belongs in Git
- environment-owned state belongs in the platform catalog and cluster runtime

### 14c. Repo-Owned vs Environment-Owned State

Good cluster architecture depends on keeping these concerns separate.

Repo-owned, git-synced state:

- pipelines, templates, scripts, styles, assets in `repo/`
- `zebflow.json`
- `zeb.lock`
- project-level bootstrap intent such as which pipeline groups should auto-activate after clone/import

Environment-owned state:

- credentials and secrets
- office placement
- live runtime leases
- MCP sessions
- invocation logs
- per-environment DB connection bindings
- any local runtime data that is not committed into Git

This means clone-from-Git should reconstruct the project source tree and repo-owned config, but not assume secrets or runtime DB contents are recoverable from Git alone.

### 14d. Three Separate Concerns

Future compute architecture should not collapse these concerns into one concept.

| Concern | Meaning |
|---|---|
| Node integration | what external system a node talks to |
| Execution backend | how an entire pipeline run executes |
| Placement policy | which computer/runtime should receive that run |

Examples:

- `n.spark.submit`, `n.hdfs.read`, `n.kafka.publish` are node-level integrations
- `resident_worker` (current internal name for office-resident execution), `k8s_job`, or
  `spark_submit` are whole-pipeline execution backends
- `local`, `pinned office`, or later `selector/pool` are placement policies

The architecture must support all three independently.

### 14e. Control Plane vs Runtime Plane

The architecture should take a firm stance here:

- `controller` is the control plane
- project runtimes are the data plane

That means the public/runtime surface should ultimately belong to the runtime side, not to the
controller.

Control-plane traffic:

- Studio UI
- project admin/settings
- project creation, clone, sync, promote, and migration
- credentials and membership management
- MCP
- preview, diagnostics, and fallback tools

Runtime-plane traffic:

- public pages
- public APIs
- webhooks
- websocket and SSE
- runtime-served `/files`
- runtime-served assets
- future runtime protocols such as MQTT

This is the architectural position Zebflow should preserve beyond `0.2.0`:

- `controller` is not the permanent hot-path ingress for production runtime traffic
- `controller` is the control lens and fallback proxy
- project runtimes are the canonical serving plane

### 14f. Secure Internal Cluster Network

Controller/office communication should be treated as an internal secure network, not as a public API
surface.

That means:

- controller/office links are expected to run only inside trusted private environments
- typical deployment shapes are localhost, office LAN, VPN, private Docker network, or Kubernetes
  cluster networking
- public internet traffic should not talk directly to internal cluster control endpoints
- control transport should harden toward mTLS and explicit node identity

This also means the product should separate:

- public ingress configuration
- internal control transport
- project-scoped runtime state transport

Even when the first implementation is simpler, the architecture must assume the secure boundary
from the start.

### 14g. Runtime Ingress Policy

Runtime ingress should be explicit and binary, not ambiguous.

Recommended vocabulary:

| Mode | Meaning |
|---|---|
| `master_proxy` | public runtime traffic enters through the controller, then the controller routes to the runtime |
| `direct_runtime` | public runtime traffic enters the project runtime directly |

The rule should be:

- control traffic always uses the controller
- runtime traffic uses either `master_proxy` or `direct_runtime`
- if `direct_runtime` is chosen, the whole runtime surface follows it

So `direct_runtime` must cover the whole runtime origin:

- pages
- APIs
- webhooks
- websocket/SSE
- runtime-served files
- runtime assets
- future runtime protocols

It should not mean "only webhook goes direct".

The recommended long-term position is:

- `master_proxy` is valid for local installs, early clusters, preview, and fallback
- `direct_runtime` is the canonical production path for serious dedicated runtimes

The extra network hop of `master_proxy` is usually small in absolute latency terms, but the more
important issue is architectural:

- the controller carries extra bandwidth
- the controller holds extra open connections
- hot project traffic competes with control/admin traffic
- the controller becomes a shared choke point

So Zebflow should evolve toward direct runtime ingress for production traffic while keeping the controller
proxy as a controlled compatibility path.

### 14h. Platform Home and Infrastructure UX

The control plane should make runtime placement visible at a glance.

Platform home and project infrastructure views should show concise runtime information such as:

- runtime mode: `shared`, `pinned`, or `dedicated`
- current office or runtime authority
- current ingress mode
- control URL
- runtime URL
- webhook base
- websocket base
- assets/files base
- office advertise address or runtime host address

They should also expose an office inventory view with:

- office id and label
- status
- advertise/base URL
- capability tags
- heartbeat freshness
- how many projects are currently placed there

This is important because operators need to see both:

- where a project is controlled
- where a project is actually served

### 14i. Base Abstractions To Lock In Now

The base needs explicit adapter seams before the distributed features expand.

Recommended core abstractions:

| Abstraction | Responsibility |
|---|---|
| `CatalogStore` | platform metadata: users, projects, credentials, placement, office registry |
| `ObjectStore` | repo/files/assets/bundles |
| `RuntimeDataStore` | per-project runtime data stores |
| `StateBus` | shared KV/pubsub/lease semantics |
| `CacheStore` | template and runtime cache |
| `ControlTransport` | secure controller/office communication |
| `ExecutionBackend` | how a whole pipeline run is executed |
| `PlacementPolicy` | where a run should go |
| `RunnerCapabilities` | tags/resources/features used for matching |
| `ExecutionHandle` | status, progress, logs, cancel for long-running work |

The clean home for these seams is:

- `src/infra/io/` for store/bus/cache interfaces and implementations
- `src/infra/cluster/` for controller/office coordination, placement, join, transport, and security

Platform should only own thin orchestration/admin pieces on top of those layers.

### 14j. First Execution Subset

The first clustered implementation should stay deliberately small.

`0.2.0` should only need:

- `ExecutionBackend::resident_local`
- `ExecutionBackend::resident_worker`
- `PlacementPolicy::local`
- `PlacementPolicy::pinned`

That means:

- no per-run pod spawning yet
- no Docker socket execution yet
- no multi-replica runner pools yet
- no direct public runtime ingress requirement in the first cluster slice
- no dedicated runtime provisioning yet

This is enough to support:

- normal SSR pages
- webhook and API traffic
- realtime pipelines
- scheduled jobs
- a simple two-node cluster

### 14k. `mem` First, Remote State Next, Shared Backends Later

For the first clustered cut, the existing in-process mem system should become the first
`StateBus` implementation.

That gives:

- standalone mode via in-memory bus
- first cluster mode with projects pinned to a single office
- a clean swap path to networked or external shared state later

This is intentionally limited:

- `mem` is sufficient when one project is pinned to one runtime
- `mem` is not enough for shared multi-runner state across offices
- later backends must handle cross-node coordination without changing the project model

So the first implementation should not pretend to be “Redis-grade” in behavior everywhere. It
should instead make the interface explicit and keep the implementation swappable.

### 14l. StateBus Access Modes

`StateBus` is for project-scoped runtime coordination, not for the platform catalog.

It should remain separate from:

- users
- projects
- credentials
- office registry
- placement records
- git/repo metadata

Those belong in durable catalog/object/runtime-data layers.

StateBus itself should support three access patterns over time:

| Mode | Meaning | Best fit |
|---|---|---|
| `MemStateBus` | in-process project state authority | standalone, single-office, project pinned to one runtime |
| `RemoteStateBus` | network client/server path to the project's state authority | small clusters, office-to-office or controller-to-office access |
| `Redis`/`Valkey`-class backend | external shared coordination backend | multi-runner hot state, cross-node fanout, shared replicas |

The intended authority model is:

- each project has one active state authority at first
- local access on that authority uses `MemStateBus`
- remote access from other nodes uses `RemoteStateBus`
- only later, when one project truly needs shared hot state across multiple active runtimes,
  should an external shared backend become necessary

That keeps the first cluster lean while still leaving room for larger shared-state topologies.

### 14m. Runtime Modes and Resource Isolation

Office does **not** mean “one office = one project”.

The correct model is:

- one office may host many project runtimes
- one project may later have one or more runners
- projects choose a runtime mode appropriate to their traffic and isolation needs

Recommended runtime modes:

| Mode | Meaning | Best fit |
|---|---|---|
| `shared` | project runs in a shared office pool | low traffic, internal tools, new projects |
| `pinned` | project always runs on one chosen office or office-class | locality, hardware affinity, predictable state ownership |
| `dedicated` | project gets its own runtime deployment or VM | high traffic, strong resource isolation, public production sites |

This matters for real deployments.

Example:

- `musiklib.org` is likely `dedicated`
- `hadaf.id` may start `shared` and later graduate
- `insanalamin.com` may remain `shared`

The architecture must allow each project to hold its own resource policy rather than forcing one
flat office model across all projects.

### 14n. Deployment Ladder

Zebflow should be able to grow through a clear deployment ladder without changing the project
model.

| Stage | Example | Typical shape | Preferred state mode | Typical runtime mode |
|---|---|---|---|---|
| 1 | Raspberry Pi in car | single `zebflow` process | `MemStateBus` | local |
| 2 | Jetson Nano in building/office | single `zebflow` process | `MemStateBus` | local |
| 3 | One laptop or one server | standalone or controller with local office | `MemStateBus` | local/shared |
| 4 | Office multi-computer with Docker only | one controller plus several offices | `MemStateBus` + `RemoteStateBus` | shared/pinned |
| 5 | Kubernetes one node | one controller plus office pods | `MemStateBus` + `RemoteStateBus` | shared/pinned/dedicated |
| 6 | Kubernetes multi-node | one controller plus office pools and dedicated runtimes | `RemoteStateBus`, later external shared backend | shared/pinned/dedicated |
| 7 | Independent project platform | standalone or separate controller/office install for one project | project chooses | dedicated or standalone |

The same repo, project bundle, and runtime profile should remain portable across these stages.

### 14o. Migration, Promotion, and Graduation

Project mobility should be a first-class product capability.

The project must be treated as a portable unit that can:

- clone into another runtime
- warm up there
- test there
- cut traffic over
- roll back if needed
- later detach into its own independent Zebflow platform

This should be visible in the product as a control-plane workflow, not hidden as an operator-only
procedure.

The intended UX is a first-class migration flow such as:

- `Clone Runtime`
- `Move Runtime`
- `Promote To Dedicated`
- `Graduate To Independent Platform`
- `Rollback`

Two migration styles should exist conceptually:

1. `clone_first`
2. `live_handoff`

The first real implementation should be `clone_first`.

That means:

1. choose a target runtime or platform
2. sync the repo-owned project bundle
3. restore optional runtime data snapshot
4. remap secrets and environment bindings
5. warm and test the target
6. cut traffic over
7. keep a rollback window

That means the architecture should preserve migration state as a real product concept rather than
forcing each deployment mode to invent its own ad hoc move procedure.

This is the correct first-class UX shape for:

- standalone device -> office server
- shared office -> pinned office
- pinned -> dedicated runtime
- one cluster -> another cluster
- federated platform -> independent platform
- dedicated runtime -> downgrade back to shared

The migration unit should be:

- `ProjectBundle`
- optional `RuntimeSnapshot`
- `SecretBindingsManifest`
- `MigrationPlan`
- `MigrationExecution`

It should **not** depend on copying the whole platform database.

### 14p. Future Execution Backends

Later workloads will need more than resident execution.

Expected future backends:

- `docker_job`
- `k8s_job`
- `spark_submit`

These are for whole-pipeline execution routing, not for ordinary web requests.

Typical fit:

- `resident_*` for web app, API, realtime, ordinary project logic
- `k8s_job` or similar for ETL, heavy batch, long-running data engineering jobs
- `spark_submit` for cluster-native Spark workloads when the whole run should execute through
  Spark infrastructure

### 14q. Data Engineering and Specialized Systems

Systems such as Spark, Hadoop, Kafka, and HDFS should fit in two ways:

1. as integration nodes
2. as execution backends when the whole pipeline needs a specialized runtime

So the architecture should support both:

- node-level integrations like `n.spark.submit`, `n.hdfs.read`, `n.kafka.publish`
- whole-pipeline routing to a backend like `spark_submit` or `k8s_job`

Those are not the same decision and should not share one overloaded setting.

### 14r. Agentic Teams

Multi-agent or agent-team behavior is primarily a project/runtime concern, not a cluster-topology
concern.

It mostly belongs in:

- pipeline design
- automaton/assistant orchestration
- project-level configuration

The compute architecture only needs to provide:

- where the agent run executes
- whether it is resident or isolated
- how progress, cancellation, and logs are surfaced

The same `ExecutionBackend` and `ExecutionHandle` abstractions should cover future agent-team
execution without requiring a separate compute model.

### 14s. Compatibility Contract Starting At `0.2.0`

`0.2.0` should become the first version with a compatibility contract for the new architecture.

From `0.2.0` onward, later versions should preserve backward compatibility for the project mobility
and deployment model wherever possible.

That means future releases should treat these as stable concepts:

- project bundle portability
- runtime mode vocabulary (`shared`, `pinned`, `dedicated`)
- execution backend vocabulary
- state bus abstraction
- migration plan / cutover model

Compatibility should prefer:

- additive config evolution
- additive schema evolution
- explicit versioned bundle and migration formats
- preserving `0.2.0` project/runtime concepts even when implementations improve

The goal is that `0.2.0` becomes the baseline that future versions extend, not a moving target
that gets reinterpreted every release.

For now, treat `docs/DISTRIBUTED_ARCHITECTURE.md` as the broader target-state vision, not as a
description of current code.

## 15. Source Map

When you need the real truth, start from these files:

| File | Why it matters |
|---|---|
| `src/lib.rs` | subsystem boundaries |
| `src/bin/zebflow.rs` | real server entrypoint and env-driven boot behavior |
| `src/infra/io/state/interface.rs` | `StateBus` contract for mem-first then Redis-style shared coordination |
| `src/infra/io/state/mem.rs` | adapter from the current `MemHub` into the new `StateBus` seam |
| `src/infra/execution/backend/interface.rs` | whole-pipeline execution backend contract |
| `src/infra/execution/placement/runtime.rs` | portable runtime mode and resource-profile vocabulary |
| `src/infra/execution/sync/bundle.rs` | versioned project bundle and secret-binding manifest |
| `src/infra/execution/sync/migration.rs` | versioned migration plan and execution model |
| `src/infra/cluster/transport/interface.rs` | future controller/office control transport contract |
| `src/platform/services/cluster/registry.rs` | current office registry persistence and control-plane office inventory |
| `src/platform/services/cluster/placement.rs` | current project placement and runtime dispatch policy |
| `src/platform/services/cluster/runtime_sync.rs` | project bundle sync and remote materialization |
| `src/platform/services/cluster/mod.rs` | product-facing cluster orchestration boundary above `infra/*` |
| `src/platform/services/access/mod.rs` | membership, invites, role presets, and user-bound git identity seam |
| `src/platform/services/access/roles.rs` | GitLab-style role preset vocabulary and managed capability bundles |
| `src/platform/services/access/git_identity.rs` | user profile -> project settings -> fallback git identity resolution |
| `src/platform/services/platform.rs` | platform composition root |
| `src/platform/services/project_config.rs` | read/write surface for repo-owned runtime/bootstrap config |
| `src/platform/web/mod.rs` | router, webhook ingress, page rendering, project APIs |
| `src/platform/web/templates/pages/home/page.tsx` | project list and create/clone runtime-target control-plane UX |
| `src/platform/web/templates/pages/project-studio/infrastructure/page.tsx` | project-facing runtime summary and office/infrastructure view |
| `src/platform/services/project.rs` | project layout, pipeline draft/activate behavior |
| `src/platform/services/pipeline_runtime.rs` | active runtime registry |
| `src/pipeline/engines/basic.rs` | graph execution semantics |
| `src/pipeline/nodes/basic/trigger/webhook.rs` | webhook trigger contract |
| `src/pipeline/nodes/basic/web_response/mod.rs` | current web response contract |
| `src/language/engines/deno_sandbox/engine.rs` | script runtime behavior |
| `src/language/runtime/tool_init.js` | shared `Tool.*` surface |
| `src/rwe/core/compiler.rs` | import rules and compile pipeline |
| `src/rwe/core/render.rs` | SSR cache, circuit breaker, hydration output |
| `src/rwe/core/deno_worker.rs` | embedded V8 worker pool |
| `src/infra/scheduler/mod.rs` | scheduled pipeline runtime |
| `src/infra/mem/subscriber.rs` | mem-subscribe runtime |
| `src/infra/transport/ws/mod.rs` | realtime room hub |
| `src/platform/services/library.rs` | `zeb/*` library registry reality |

If this file and the code disagree, the code wins.
