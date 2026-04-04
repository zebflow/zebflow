# Zebflow Distributed Architecture

> **Status: Vision + Spec — not yet implemented**
> Current codebase is Phase 0 (monolith with all Local providers).
> This document defines the target architecture and the migration path.

---

## 0. Core Principle — Act Locally, Think Globally

> The same binary that runs on a developer's laptop is the binary that runs in a K8s pod.
> The only difference is configuration.

No prod-only code paths. No "simplified local mode." What you test locally is exactly what runs in production. Infrastructure is injected at startup — the business logic never asks "where am I running?"

---

## 1. The Two Planes

```
┌─────────────────────────────────────────────────────┐
│  PLATFORM  (Control Plane)                          │
│                                                     │
│  - Project registry (knows where each project is)  │
│  - User auth, billing, routing table                │
│  - UI (Zebflow Studio)                              │
│  - MCP gateway (routes to the right project)        │
│  - Like: LENS for K8s, Vercel Dashboard             │
└─────────────────────┬───────────────────────────────┘
                      │  (address lookup + proxy/route)
          ┌───────────┼───────────┐
          ↓           ↓           ↓
    [Project A]  [Project B]  [Project C]
    LocalInst.   ContainerInst  K8sInst
```

The Platform is a **thin controller** — it knows what exists and where it lives. It does not run pipelines, compile templates, or execute JS. That is the Project's job.

---

## 2. ProjectInstance — The Independent Unit

Each project is a **self-contained deployable unit**. It exposes one interface to the outside world. The Platform calls that interface — it does not care what is underneath.

```
ProjectInstance (interface)
  execute_pipeline(graph, input) → output
  compile_template(source) → compiled
  handle_http(request) → response
  health() → status
  notify_file_changed(rel_path) → ()
```

### Variants

| Variant | Description | When used |
|---------|-------------|-----------|
| `LocalProjectInstance` | Runs inside the platform process. Zero network hop. | Single-install, dev |
| `ContainerProjectInstance` | Separate Docker container. Address stored in platform. | Self-hosted multi-project |
| `K8sProjectInstance` | K8s Deployment with Service. Address = K8s DNS. | Production, cloud |

In `LocalProjectInstance`, "calling" the project is a direct Rust function call — no serialization, no network. In container/K8s variants, it becomes an HTTP or gRPC call to the project's address.

---

## 3. ProjectInstance Internal Components

Every ProjectInstance, regardless of variant, is composed of the same set of provider interfaces:

```
ProjectInstance
  ├── DBInstance          — persistent structured data
  ├── ObjectInstance      — files, assets, blobs
  ├── PubSubInstance      — real-time events, cross-runner messaging
  ├── CacheInstance       — compiled template cache, hot data
  └── Runner(s)           — actual pipeline/template execution
```

### DBInstance

| Provider | Description | Config |
|----------|-------------|--------|
| `SqliteDB` | Local file-based SQLite | `db.path` |
| `PostgresDB` | PostgreSQL (or CockroachDB, etc.) | `db.url` |
| `MySQLDB` | MySQL / PlanetScale | `db.url` |

### ObjectInstance

| Provider | Description | Config |
|----------|-------------|--------|
| `LocalFS` | Local filesystem directory | `object.path` |
| `S3Object` | AWS S3 / R2 / MinIO | `object.bucket`, `object.endpoint` |
| `GCSObject` | Google Cloud Storage | `object.bucket` |

### PubSubInstance

| Provider | Description | When |
|----------|-------------|------|
| `InMemoryPubSub` | tokio broadcast, process-local | Single runner (LocalProjectInstance) |
| `RedisPubSub` | Redis Pub/Sub | Multiple runners, cross-pod WebSocket sync |
| `NATSPubSub` | NATS JetStream | High-throughput event streaming |

### CacheInstance

| Provider | Description | When |
|----------|-------------|------|
| `InMemoryCache` | HashMap per project, process-local | Single runner |
| `RedisCache` | Redis with project-namespaced keys | Multiple runners |

### Runner

| Kind | Description | When |
|------|-------------|------|
| `LocalRunner` | In-process, zero-copy | Default in LocalProjectInstance |
| `DockerRunner` | Separate container, HTTP/gRPC | ContainerProjectInstance |
| `PodRunner` | K8s pod, HTTP/gRPC | K8sProjectInstance |

A ProjectInstance can have **multiple runners** — the controller load-balances across them (round-robin or least-connections). This is how horizontal scaling works: more pods = more runners registered with the controller.

---

## 4. The "Same Binary" Principle

The project binary is the same artifact whether running as LocalRunner or as a DockerRunner. The startup config determines which providers are wired up:

```toml
# local.toml (default, bundled single-install)
[db]      kind = "sqlite",    path = "./data/project.db"
[object]  kind = "localfs",   path = "./repo"
[pubsub]  kind = "memory"
[cache]   kind = "memory"

# production.toml (Docker / K8s — injected via env or configmap)
[db]      kind = "postgres",  url  = "$DATABASE_URL"
[object]  kind = "s3",        bucket = "$S3_BUCKET"
[pubsub]  kind = "redis",     url  = "$REDIS_URL"
[cache]   kind = "redis",     url  = "$REDIS_URL"
```

`docker build` produces an image. The image is config-agnostic. The K8s ConfigMap or Docker env vars inject the provider config at runtime.

---

## 5. K8s Scaling Model

```
[Ingress / LoadBalancer]
        ↓  round-robin
┌──────────┐  ┌──────────┐  ┌──────────┐
│  Pod 1   │  │  Pod 2   │  │  Pod 3   │   ← same image, same binary
│ Runner   │  │ Runner   │  │ Runner   │
└──────────┘  └──────────┘  └──────────┘
      └─────────────┬──────────────┘
                    ↓
      ┌─────────────┼─────────────┐
   [PostgreSQL]  [Redis]         [S3]
   persistent    cache+pubsub    files
   data          websocket sync
                 leader election
                 job queues
```

### The three horizontal scaling problems — solved

| Problem | Solution |
|---------|----------|
| In-memory cache stale across pods | `RedisCacheInstance` — all pods share one cache namespace, eviction broadcasts via PubSub |
| WebSocket rooms per-pod | `RedisPubSubInstance` — pod 1 publishes, all pods receive, each forwards to local connections |
| Cron fires N times | Leader election via Redis `SETNX` — only one pod runs the scheduler at a time |

---

## 6. Communication: Control vs Data Flow

Two distinct flows with different requirements:

### Control flow (low frequency, can afford latency)
```
Platform → ProjectControllerNode → Runner
```
Used for: MCP tool calls, config changes, file notifications, deploy operations.
Protocol: HTTP/gRPC, authenticated.

### Data flow (high frequency, latency sensitive)
```
[Ingress] → Runner directly
```
Used for: webhook HTTP requests, WebSocket connections, page renders.
Protocol: HTTP. The Ingress routes by project hostname/path directly to the runner pods — no proxy hop through the controller.

The controller is **not in the hot path** for user-facing requests.

---

## 7. File Change & Cache Invalidation

When a template file changes (via UI, MCP, git push, API — any path):

```
File write
    ↓
notify_file_changed(rel_path)   ← ProjectInstance method, all callers go through this
    ↓
resolve absolute path
    ↓
PubSubInstance.publish("cache:evict", { path: abs_path, project: id })
    ↓
all Runners subscribed to this channel:
    CacheInstance.evict_by_dependency(abs_path)
    ← removes all cache entries whose dependency set contains abs_path
```

In `LocalProjectInstance` (single runner, InMemoryPubSub): the publish and subscribe happen in the same process — it's just a function call. Zero overhead.

In K8s (multiple pods, RedisPubSub): Redis fan-out delivers the eviction event to all pods simultaneously. Each pod evicts its local entries.

The cache entry stores its dependency set at compile time — the compiler already knows every file it read (the `visited` set in `collect_inlined_module`). No extra analysis needed.

---

## 8. Current State vs Target

### Phase 0 — Monolith (current)
```
One binary. One process. All projects. All Local* providers hardwired.
No ProjectInstance concept — everything is flat in PlatformAppState.
```

### Phase 1 — Introduce interfaces without moving anything
- Define `ProjectInstance` trait
- Define `DBInstance`, `ObjectInstance`, `PubSubInstance`, `CacheInstance`, `Runner` traits
- Implement all `Local*` variants (wrapping current code)
- Wire through dependency injection — behavior identical to Phase 0
- **Ship: zero visible change, but seams are now explicit**

### Phase 2 — Cache eviction (immediate next task)
- Add `dependency_paths: HashSet<String>` to `CompiledTemplate`
- `CacheEntry` stores `(CompiledPage, dependency_paths)`
- File write triggers `evict_by_dependency(abs_path)` on `InMemoryCache`
- Remove the blunt `.clear()` from `api_template_save`
- **Ship: correct incremental cache invalidation for all write paths**

### Phase 3 — Project isolation
- Each project gets its own `LocalProjectInstance`
- Cache is per-project (no cross-project collision possible)
- `PlatformService` holds a `HashMap<ProjectId, Arc<dyn ProjectInstance>>`
- **Ship: explicit project boundaries, ready for extraction**

### Phase 4 — Container/K8s ProjectInstance
- `ContainerProjectInstance` — holds an HTTP address, proxies calls
- `K8sProjectInstance` — holds a K8s service DNS name
- Platform can register remote projects in addition to local ones
- **Ship: multi-project deployments, each project in its own container**

### Phase 5 — Distributed providers
- `RedisCacheInstance`, `RedisPubSubInstance`
- `PostgresDB`, `S3Object`
- Leader election for scheduler
- **Ship: horizontally scalable project deployments**

---

## 9. The Docker Image

The project image packages:
- The Zebflow project runner binary (same code as the local install)
- Default `production.toml` expecting env vars for all providers
- No project-specific code — project files are loaded from `ObjectInstance` at startup or via git clone

```dockerfile
FROM zebflow/runner:latest
ENV DB_KIND=postgres
ENV PUBSUB_KIND=redis
ENV CACHE_KIND=redis
ENV OBJECT_KIND=s3
# actual credentials injected via K8s secrets
```

The platform builds and pushes this image on deploy. K8s pulls it, starts the pod, the runner connects to the configured providers and registers with the platform.

---

## 10. Key Design Constraints

1. **No special-casing for local.** `LocalRunner` is not a simplified version — it is a full implementation that happens to use in-process communication.
2. **Business logic is provider-agnostic.** Pipeline execution, template compilation, Deno JS — none of this code knows if the DB is SQLite or Postgres.
3. **`notify_file_changed` is the single choke point** for any file mutation. All write paths (UI, MCP, API, git) call this. Cache invalidation, index updates, and hot-reload all hang off this one hook.
4. **The platform is thin.** It routes. It does not compute.
5. **Projects are independently deployable.** A project can be upgraded, restarted, or scaled without touching the platform or other projects.
