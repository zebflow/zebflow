# Zebflow Platform Architecture

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      ZebflowEngineKit                           │
│                        (src/lib.rs)                             │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │   pipeline   │  │   language   │  │        rwe           │  │
│  │              │  │              │  │                      │  │
│  │ graph BFS    │  │ Deno sandbox │  │  TSX compile + SSR   │  │
│  │ node dispatch│  │ script nodes │  │  client hydration    │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────────────────┘  │
│         │                 │                                     │
│         └─────────────────┘                                     │
│                   ↑ used by                                     │
│  ┌────────────────┴────────────────────────────────────────┐    │
│  │                      platform                           │    │
│  │   Axum web server · services · MCP · DSL shell          │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                 │
│  ┌──────────────┐  ┌──────────────────────────────────────┐    │
│  │   automaton  │  │               infra                  │    │
│  │  Zebtune+LLM │  │  transport/ws  storage  scheduler    │    │
│  │  agentic loop│  │  (WsHub)      (stubs)   (stubs)     │    │
│  └──────────────┘  └──────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

```
src/
├── lib.rs               ZebflowEngineKit composition root
├── pipeline/            PipelineEngine trait, BasicPipelineEngine, nodes, DSL model
│   ├── interface.rs     PipelineEngine trait
│   ├── model.rs         PipelineGraph, PipelineNode, PipelineContext, PipelineError
│   ├── nodes/
│   │   ├── interface.rs NodeHandler trait (execute_async)
│   │   └── basic/       all built-in node kinds
│   ├── engines/
│   │   └── basic.rs     BasicPipelineEngine (BFS traversal, node dispatch, merge logic)
│   └── registry.rs      PipelineEngineRegistry
├── language/            DenoSandboxEngine — n.script node execution
├── rwe/                 TSX compile (OXC) + SSR (deno_core) + client hydration
│   ├── core/compiler.rs TSX parse, import resolve, server+client module split
│   ├── core/render.rs   SSR render, client module bootstrap, HTML shell assembly
│   ├── core/deno_worker.rs  singleton V8 thread (deno_core 0.390)
│   └── engines/rwe.rs   RweReactiveWebEngine (implements ReactiveWebEngine trait)
├── automaton/           Zebtune REPL, agentic loop, LLM clients
├── platform/            Axum server, services, MCP, DSL shell
│   ├── web/mod.rs       all routes + webhook + ws handlers
│   └── services/        PlatformService composition root
└── infra/
    └── transport/ws/    WsHub, RoomHandle, RoomCmd
```

---

## 2. HTTP Routes

```
GET  /                               → redirect to /home
GET  /login          POST /login     → login page + submit
POST /logout
GET  /home
GET  /design-system

GET  /projects/{owner}/{project}     → project root (redirects to /pipelines/registry)
GET  /projects/{owner}/{project}/pipelines/{tab}
GET  /projects/{owner}/{project}/build/{tab}     ← template editor
GET  /projects/{owner}/{project}/credentials
GET  /projects/{owner}/{project}/db/connections
GET  /projects/{owner}/{project}/db/{kind}/{conn}/{tab}
GET  /projects/{owner}/{project}/settings/{tab}
GET  /projects/{owner}/{project}/dashboard

GET|POST /api/projects/{owner}/{project}/*    ← CRUD, pipeline, git, MCP, DB, assistant
GET|POST /api/users
GET|POST /api/admin/db/*             ← superadmin only

ANY  /wh/{owner}/{project}/{*tail}   ← webhook ingress (pipeline trigger)
GET  /ws/{owner}/{project}/rooms/{room_id}  ← WebSocket upgrade

GET  /assets/branding/{asset}
GET  /assets/platform/{asset}
GET  /assets/libraries/{*path}
GET  /assets/rwe/scripts/{hash}                  ← platform page client JS
GET  /assets/{owner}/{project}/rwe/scripts/{hash} ← project page client JS

/api/projects/{owner}/{project}/mcp  ← MCP protocol (nested router)
```

All platform pages (login, home, project pages) are pre-compiled at startup by
`build_frontend()` → `compile_page()` → `rwe.compile_template()`. Results stored
in `PlatformFrontend.pages` (BTreeMap). Rendering at request time is SSR-only.

---

## 3. Webhook Pipeline Flow

```
ANY /wh/{owner}/{project}/{*tail}
        │
        ▼ public_webhook_ingress (src/platform/web/mod.rs)
        │
        ├── pipeline_runtime.list_project(owner, project)
        │       → all active CompiledPipeline entries for this project
        │
        ├── Score + filter by n.trigger.webhook:
        │       method match (case-insensitive)
        │       path match (static segments, :param, wildcard)
        │       sort: static_segments DESC, dynamic_segments ASC, total_segments DESC
        │       → select highest-scoring candidate
        │
        ├── No candidate → dispatch_weberror(404) → or 404 JSON
        │
        ├── verify_webhook_auth(headers, body, auth_type, auth_credential)
        │       auth_type = "none" → pass
        │       auth_type = "bearer" → check Authorization header vs stored secret
        │       auth_type = "jwt" → verify JWT, extract claims → injected as input.auth
        │
        ├── hydrate_web_render_markup_from_templates(state, owner, project, &mut graph)
        │       for each n.web.render node in graph:
        │           read node.config.template_path → resolve file on disk
        │           projects.read_template_file() → string
        │           node.config.markup = file contents
        │
        ├── apply_rwe_project_options(state, owner, project, &mut graph)
        │       for each n.web.render node in graph:
        │           read zebflow.json → rwe section (minify_html, strict_mode, allow_list)
        │           resolve project template_root path
        │           parse node-level --load-scripts CSV → Vec<String>
        │           inject full ReactiveWebOptions JSON into node.config.options
        │
        ├── build_webhook_ingress_input() → JSON input:
        │       { method, path, query: {...}, headers: {...}, body: {...|raw},
        │         params: {:param → value}, auth: {...} (if JWT) }
        │
        ├── BasicPipelineEngine::new(language, rwe, credentials, simple_tables)
        │       .with_web_render_cache(state.web_render_cache)
        │       NOTE: no ws_hub — webhook pipelines cannot use ws nodes
        │
        ├── engine.execute_async(&graph, &ctx) → PipelineOutput { value, trace }
        │       (see §4: Pipeline Engine Execution)
        │
        ├── record_success / log_pipeline_invocation
        │
        └── Response dispatch:
                output.value has "html" key
                    → inject CSS as <style data-rwe-tw> before </head>
                    → externalize_rwe_scripts: store JS, serve from
                       /assets/{owner}/{project}/rwe/scripts/{hash}
                    → Html(html).into_response()  [Content-Type: text/html]

                output.value has "_status" key
                    → HTTP status = _status value
                    → if >= 400: dispatch_weberror(status, body) or (status, JSON)
                    → if < 400: (status, JSON body without _status)

                else → Json({ ok: true, output: value, trace })
```

---

## 4. Pipeline Engine Execution (BasicPipelineEngine)

```
BasicPipelineEngine.execute_async(graph, ctx)
        │
        ├── validate_graph: check entry_nodes exist, edges valid, all node kinds supported
        │
        ├── Build outgoing edge map: (from_node, from_pin) → [(to_node, to_pin)]
        │
        ├── Initialize BFS queue with entry_nodes (or graph.nodes[0])
        │   each QueueEntry has: node_id, input_pin, payload, metadata
        │
        └── Loop: queue.pop_front()
                │
                ├── build_node(node) → NodeDispatch variant
                │       deserializes node.config into typed Config struct
                │       attaches services (credentials, simple_tables, ws_hub, language, rwe)
                │
                ├── dispatch.execute_async(input) → NodeOutput { payload, output_pins, trace }
                │
                │   Node kinds:
                │   n.trigger.webhook    pass-through; output_pins = ["out"]
                │   n.trigger.ws         pass-through; output_pins = ["out"]
                │   n.trigger.manual     pass-through; output_pins = ["out"]
                │   n.script             DenoSandboxEngine.execute(code, input)
                │   n.http.request       outbound HTTP via reqwest
                │   n.pg.query           PostgreSQL via CredentialService → rows JSON
                │   n.sjtable.query      SekejapDB via SimpleTableService → rows JSON
                │   n.web.render         compile + SSR (see §6)
                │   n.ws.emit            WsHub.send_cmd(Emit { event, to, payload })
                │   n.ws.sync_state      WsHub.send_cmd(PatchState { op, path, value })
                │   n.trigger.weberror   matches on status_code; pass-through
                │   n.auth_token.create  JWT sign via CredentialService
                │   n.crypto.*           hash / encrypt
                │   n.logic.if           evaluate condition → "true"/"false" output_pin
                │   n.logic.switch       multi-branch condition → named output_pin
                │   n.logic.branch       split payload to multiple downstream pins
                │   n.logic.merge        wait_all / first_completed / pass_through
                │   n.zebtune            LLM call (client_from_env)
                │
                ├── last_value = output.payload
                │
                └── For each emitted output_pin:
                        find outgoing edges → enqueue target nodes
                        merge node handling:
                            wait_all: accumulate per-pin payloads; fire when all pins received
                            first_completed: fire once, ignore subsequent arrivals
                            pass_through: enqueue immediately (default)

        Returns PipelineOutput { value: last_value, trace }
```

---

## 5. WebSocket Pipeline Flow

```
GET /ws/{owner}/{project}/rooms/{room_id}  (WebSocket upgrade)
        │
        ▼ ws_room_handler → handle_ws_room
        │
        ├── session_id = "ws-{nanos:016x}"
        │
        ├── WsHub.get_or_create_room("{owner}/{project}/{room_id}") → Arc<RoomHandle>
        │       RoomHandle holds: state: Arc<RwLock<Value>>, broadcast: tokio broadcast channel
        │
        ├── room.subscribe() → tokio broadcast Receiver<String>
        ├── room.join_session() → RAII SessionGuard (decrements count on drop)
        ├── room.get_state() → current state snapshot (Value)
        │
        ├── Send initial message to client:
        │       { "type": "joined", "session_id": "...", "room": "...", "state": {...} }
        │
        └── tokio::select! loop
                ├── broadcast_rx.recv()
                │       → forward JSON string to client as WS Text message
                │       → RecvError::Closed → break
                │       → RecvError::Lagged → skip, continue
                │
                └── socket.recv()
                        → Message::Text(json) → parse { event, payload }
                        │       ws_dispatch_event(owner, project, room_id, session_id,
                        │                         event, payload, state)
                        │           (see below)
                        │
                        → Message::Close | None → break

        // After loop:
        WsHub.remove_room(room_key)   ← cleans up room if now empty

─────────────────────────────────────────────────────────────────────
ws_dispatch_event(owner, project, room_id, session_id, event, payload)
        │
        ├── pipeline_runtime.list_project(owner, project)
        │       → all active CompiledPipelines
        │
        ├── Filter: p.ws_triggers.any(|t|
        │       (t.room.is_empty() || t.room == room_id)
        │       && (t.event.is_empty() || t.event == event)
        │   )
        │       WsTriggerSpec { node_id, room, event } extracted from n.trigger.ws nodes
        │       at pipeline activation time
        │
        └── For each matching pipeline:
                input = { room_id, session_id, event, payload }
                ctx = PipelineContext { owner, project, pipeline, request_id: "ws-{...}" }

                tokio::spawn(async {
                    BasicPipelineEngine::new(language, rwe, credentials, simple_tables)
                        .with_ws_hub(ws_hub)   ← ws_hub required for ws nodes
                        .execute_async(&graph, &ctx)
                })
                NOTE: fire-and-forget; WS response is via room broadcast, not return value

─────────────────────────────────────────────────────────────────────
WS client protocol:

  Client → Server:  { "event": "<name>", "payload": {...} }

  Server → Client:
    { "type": "joined", "session_id": "...", "room": "...", "state": {...} }
    { "type": "state_patch", ... }   ← emitted by n.ws.sync_state broadcast
    { "type": "event", "event": "<name>", "payload": {...} }  ← emitted by n.ws.emit
```

---

## 6. n.web.render Node + RWE Flow

```
Within BasicPipelineEngine — InlineWebRender dispatch:
        │
        ├── markup = node.config.markup  (hydrated earlier from template file)
        ├── hash_markup(markup) → u64 cache key
        ├── WebRenderCache lookup (Arc<Mutex<HashMap<u64, Arc<Compiled>>>>)
        │
        ├── Cache MISS → web_render::Node::compile(node_id, config, TemplateSource, rwe, language)
        │       └── RweReactiveWebEngine.compile_template(source, language, options)
        │               options.templates.template_root = project templates/ dir
        │               options.allow_list = zebflow.json rwe allow_list
        │               options.processors = ["tailwind"]
        │               └── crate::rwe::core::compile(markup, CompileOptions)
        │                       ├── OXC: parse TSX → JS AST
        │                       ├── resolve @/ imports → absolute paths using template_root
        │                       ├── strip `import { ... } from "rwe"` (entry file only)
        │                       ├── read imported component files from disk (inline)
        │                       ├── build server_module_source (full component tree)
        │                       ├── build client_module_source (same, for browser)
        │                       └── returns CompiledTemplate {
        │                               server_module_source,  // JS for SSR via Deno
        │                               client_module_source,  // JS for browser hydration
        │                               hydrate_mode,
        │                               deno_timeout_ms: 3000
        │                           }
        │
        ├── Cache STORE → cache.insert(hash, Arc::new(compiled))
        │
        └── web_render::render_with_engines(compiled, payload, metadata, rwe, language, request_id)
                └── crate::rwe::core::render(compiled, vars)
                        │
                        ├── SSR: deno_worker::render_ssr(server_module_source, vars, timeout_ms)
                        │       │
                        │       └── send JsOp::RenderSsr via mpsc to singleton JS thread
                        │               Singleton JS thread (one JsRuntime, one tokio executor):
                        │               ├── execute_script: globalThis.input = ctx (JSON)
                        │               ├── load_side_es_module(source) → run component
                        │               │       installGlobals() at startup:
                        │               │           globalThis.h, Fragment, React
                        │               │           globalThis.useState, useEffect, useRef,
                        │               │           useMemo, usePageState, cx, Link
                        │               │       renderToString(<Page input={...} />) called in JS
                        │               └── op_rwe_store_result(html) → thread-local slot
                        │               Returns SsrResult { html, page_config }
                        │
                        ├── Client: transpile_client_cached(client_module_source)
                        │       strip_rwe_client_imports (strip "rwe" imports)
                        │       deno_worker::transpile_client (Deno bundler, cached 256 entries)
                        │
                        ├── build_client_module(transpiled):
                        │       ├── replace npm:preact → https://esm.sh/preact@10.28.4
                        │       ├── install preact globals on globalThis:
                        │       │       h, Fragment, React, useState, useEffect, useRef,
                        │       │       useMemo, usePageState, useNavigate, Link, cx
                        │       ├── install SPA navigator (rweNavigate + progress bar)
                        │       ├── base64 encode user component code
                        │       └── bootstrap: hydrate(h(__RweRoot, __input),
                        │                              document.getElementById('__rwe_root'))
                        │
                        └── Assemble HTML:
                                body_content =
                                  <div id="__rwe_root">{SSR html}</div>
                                  <script type="application/json" id="__rwe_payload">{vars}</script>

                                build_document_shell(page_config, body_content):
                                  <!DOCTYPE html>
                                  <html lang="{page.html.lang}">
                                  <head>
                                    <meta charset="utf-8">
                                    <meta name="viewport" ...>
                                    <title>{page.head.title}</title>
                                    <meta name="description" ...>
                                  </head>
                                  <body class="{page.body.className}">
                                    {body_content}
                                  </body>
                                  </html>

                                Returns RenderOutput { html, js: client_module_js, ... }

Back in webhook handler:
  CSS from hydration_payload.css → inject as <style data-rwe-tw> before </head>
  JS blob → externalize_rwe_scripts → saved, served from:
      /assets/{owner}/{project}/rwe/scripts/{hash}   (project pipelines)
      /assets/rwe/scripts/{hash}                      (platform pages)
  Html(html).into_response()

IMPORTANT — n.web.render compile cache:
  Key = hash_markup(markup)  (hash of TSX source string)
  Shared across all webhook requests within one server run.
  Invalidated only when template file content changes (new hash on next request).
  Deno module cache is per-server-run (singleton thread); restart to clear.
```

---

## 7. SPA Navigation

```
User clicks <Link href="/x">  (or window.rweNavigate("/x"))
        │
        ├── Show progress bar (0% → 30%)
        ├── fetch("/x", { credentials: "same-origin" })
        ├── r.ok == false → bar fail, window.location.href = href (full reload)
        ├── Parse response HTML via DOMParser
        ├── document.getElementById("__rwe_root").innerHTML = newRoot.innerHTML
        ├── document.getElementById("__rwe_payload").textContent = newPayload
        ├── Remove old <style data-rwe-tw>, insert new ones from response <head>
        ├── Swap <link rel="stylesheet"> tags
        ├── document.body.className = doc.body.className
        ├── document.documentElement.lang = doc.documentElement.lang
        ├── Remove old <script data-rwe-nav-script> elements
        ├── Fetch + inject new <script type="module"> elements
        ├── document.title = doc.title
        ├── history.pushState(null, "", href)
        ├── window.scrollTo(0, 0)
        ├── Promise.all(scripts) → window.dispatchEvent("rwe:nav", { url: href })
        └── Progress bar complete

window.popstate → rweNavigate(location.pathname + location.search)

IMPORTANT:
  Old Preact tree is NOT unmounted (no preact.unmount()).
  Components with fixed/overlay elements (e.g. GitPanel) MUST listen for "rwe:nav"
  and close themselves to avoid DOM leaks into the next page.

  Console panel is teleported to document.body (outside __rwe_root) —
  it survives SPA navigation by design.
```

---

## 8. DSL Shell

```
DSL text: "| trigger.webhook --path /blog | pg.query --credential db -- SELECT ..."
        │
        ▼ src/platform/shell/parser.rs
        │
        ├── expand_kind():       "trigger.webhook" → "n.trigger.webhook"
        ├── parse_node_config(): --flag → config JSON key
        │       --path         → "path"
        │       --method       → "method"
        │       --credential   → "credential_id"
        │       --template     → "template_id" + "template_path"  (both set)
        │       --route        → "route"
        │       --room         → "room"
        │       --event        → "event"
        │       --op           → "op"
        │       --to           → "to"
        │       -- SQL text    → "sql"
        └── builds PipelineGraph { nodes, edges }
        │
        ├──→ register_pipeline → saved to repo/pipelines/*.zf.json  (status: draft)
        │       activate_pipeline → PipelineRuntimeService.activate() → goes live
        │
        └──→ run_ephemeral → BasicPipelineEngine.execute() directly (not saved)
```

---

## 9. Data Layers

```
Layer 1 — Platform Catalog
  data/platform/catalog/  (SekejapDB)
      users, auth sessions, MCP sessions, credentials, pipeline hits, invocation log
      written by: PlatformService (UserService, AuthService, McpSessionService, etc.)

Layer 2 — Project Config
  repo/zebflow.json
      project title, assistant LLM settings, rwe options (allow_list, minify_html…)
  repo/pipelines/**/*.zf.json
      pipeline definitions (graph + nodes + edges)
  repo/templates/**/*.tsx
      template source files (pages, components, layouts, behaviors, styles)
      written by: ZebflowJsonService, template file APIs

Layer 3 — Project Data
  data/sekejap/  (SekejapDB)
      SjTable rows, agent docs (AGENTS.md, MEMORY.md), invocation log
      written by: SimpleTableService
```

---

## 10. Platform Services

```
PlatformService  (src/platform/services/platform.rs)
  — composition root, Arc<PlatformService> in every Axum handler via State<PlatformAppState>
  │
  ├── PipelineRuntimeService   load+activate graphs, find matching webhook/WS triggers
  ├── ProjectService           project CRUD, template file read/write, template_root
  ├── UserService              user CRUD + password auth
  ├── AuthService              session cookie issuance + validation
  ├── CredentialService        encrypted secrets storage + retrieval
  ├── SimpleTableService       SjTable CRUD via SekejapDB
  ├── DbConnectionService      PostgreSQL/MySQL connection registry
  ├── DbRuntimeService         execute SQL against named connections
  ├── McpSessionService        MCP session create/lookup/expire
  ├── ZebflowJsonService       zebflow.json read/write
  ├── AssistantConfigService   LLM settings (reads zebflow.json)
  ├── AssistantPlatformTools   15 tools for project assistant agentic loop
  ├── PipelineHitsService      per-pipeline success/failure counters
  └── WsHub                    in-memory WebSocket room registry
      (src/infra/transport/ws/)
```

---

## 11. Template Import Rules

```
Entry page (.tsx file that n.web.render directly processes):
  ✓ import { useState, useEffect, useRef, useMemo, cx, Link } from "rwe"
      → STRIPPED at compile time; these are globalThis globals in both SSR and client

  ✓ import Button from "@/components/ui/button"
      → @/ resolved to project templates/ root at compile time

Imported component files (components/, layout/):
  ✗ DO NOT import hooks from "rwe" — they are already globals
  ✗ DO NOT import from "npm:preact/hooks" or "npm:preact"
  ✓ Just USE useState, useEffect, etc. directly — they are on globalThis

NEVER:
  ✗ import { render } from "npm:preact" → never call render() manually
  ✗ relative imports (../../components/...) — use @/ alias
  ✗ CSS variable colors like bg-indigo-600 in user project templates
      → platform's --zebflow-color-* vars are not available in user project context
      → use slate-* or explicit Tailwind core colors instead

CSS generation rule:
  RWE generates CSS only from class names present in the SSR render output.
  Conditional branches NOT taken at SSR time get NO CSS.
  Pattern: render both branches always, toggle with "hidden" class so all
  Tailwind classes appear in SSR output and get CSS generated.
```

---

## 12. Where to Look When Things Break

```
ERROR                                         WHERE TO LOOK
─────────────────────────────────────────────────────────────────────────
PLATFORM_TEMPLATE_MISSING                     platform/services/project.rs
  template file 'pages/x' not found           resolve_template_entry() requires
                                              explicit .tsx extension

FW_NODE_WEB_RENDER_COMPILE:                   rwe/engines/rwe.rs + core/compiler.rs
  template_root required for @/ import        apply_rwe_project_options() must have run;
                                              user project needs components/ui/ in templates/

SyntaxError: Identifier 'X' already declared  Two component files define same function at
                                              module scope. core/compiler.rs inlines all
                                              imports → collision. Rename the function.

SSR renders blank / missing content           rwe/core/deno_worker.rs
  after client hydration                      CSS not generated: render both branches at
                                              SSR time; toggle visibility with "hidden"

bg-indigo-600 / custom color renders nothing  --zebflow-color-* CSS vars defined in
                                              platform main.css only. User project
                                              templates must use core Tailwind colors.

WS events not dispatching to pipeline         ws_dispatch_event() in web/mod.rs
                                              Check WsTriggerSpec on compiled pipeline:
                                              n.trigger.ws config must set room+event.
                                              WsHub at src/infra/transport/ws/

Template changes not reflected after save     WebRenderCache keyed by markup hash.
                                              Deno module cache is per-server-run.
                                              Restart server to clear module cache.

Overlay/backdrop stuck after SPA nav          Fixed-position component not listening for
                                              "rwe:nav" event. Add useEffect with
                                              window.addEventListener("rwe:nav", close).

FW_NODE_WS_SYNC_STATE_UNAVAILABLE            ws_hub not attached to engine.
  / FW_NODE_WS_EMIT_UNAVAILABLE              Webhook pipelines don't get ws_hub.
                                              WS nodes only usable in WS pipelines.
```
