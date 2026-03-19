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
│   └── runtime/
│       └── tool_init.js  Tool.* standard library — shared by n.script AND all RWE contexts
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

GET    /api/projects/{owner}/{project}/templates/pages  ← list .tsx files; ?path= prefix filter
ANY    /api/projects/{owner}/{project}/mcp              ← MCP protocol (nested router, see §19)
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

                else → Json(output.value_without_set_cookie).into_response()
                    _set_cookie stripped from body → written as Set-Cookie header
                    Pipeline output is the HTTP response body, no envelope.
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
                │   n.sjtable.mutate     SekejapDB row-level mutations (delete)
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

### 6a. RWE Source File Map

```
src/rwe/
├── core/
│   ├── compiler.rs         Phase 1 — OXC parse, @/ import resolution, bundle_for_client
│   ├── deno_worker.rs      Phase 2a — singleton V8 thread (embedded deno_core 0.390)
│   │                                  SSR render + client transpile cache
│   └── render.rs           Phase 2b — SSR orchestration, client bootstrap assembly,
│                                       HTML shell construction, WebRenderCache
├── engines/
│   └── rwe.rs              RweReactiveWebEngine (implements ReactiveWebEngine trait)
└── runtime/
    ├── preact_ssr_init.js  SSR globals file — executed ONCE at JsRuntime startup
    │                       installs all hooks + helpers as globalThis.*
    └── ssr_worker.mjs      NOT USED — legacy external Deno subprocess (do not edit)
```

### 6b. Pipeline Dispatch → Compile → Render

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
        │                       ├── security::analyze() — blocklist check on import URLs
        │                       ├── rewrite_imports() — @/ → file:// absolute paths
        │                       ├── rewrite_page_root_tag() — <Page> → <>
        │                       ├── bundle_for_client():
        │                       │       recursively reads each imported component file
        │                       │       inlines all into a single flat JS string
        │                       │       "zeb" imports remain verbatim at this stage
        │                       │       prefix_module_locals() prefixes UPPER_SNAKE_CASE
        │                       │       consts to avoid collision across inlined modules
        │                       ├── prepend `/** @jsxImportSource npm:preact */\n`
        │                       └── returns CompiledTemplate {
        │                               server_module_source,  // full bundle for SSR
        │                               client_module_source,  // same bundle for browser
        │                               hydrate_mode,
        │                               deno_timeout_ms: 3000
        │                           }
        │
        ├── Cache STORE → cache.insert(hash, Arc::new(compiled))
        │
        └── web_render::render_with_engines(compiled, payload, metadata, rwe, language, request_id)
                └── crate::rwe::core::render(compiled, vars)
                        │
                        ├── Phase 2a — SSR (deno_worker.rs):
                        │       deno_worker::render_ssr(server_module_source, vars, timeout_ms)
                        │       │
                        │       └── send JsOp::RenderSsr via mpsc → singleton JS thread
                        │               One JsRuntime, one dedicated tokio executor.
                        │               preact_ssr_init.js executed ONCE at startup.
                        │               Per render:
                        │               1. transpile_tsx(source) — OXC TSX→JS
                        │               2. strip_zeb_imports() — remove "zeb" import lines
                        │                  NOTE: strips from entire bundle (entry + all inlined
                        │                  components) in one pass; hooks are globalThis globals
                        │               3. inject_page_globals() — scan for `export default function`
                        │                  append globalThis.__rwe_page + __rwe_page_config
                        │               4. execute_script: `globalThis.input = {ctx JSON}`
                        │               5. write temp file → load_side_es_module(file://)
                        │                  (MUST use load_side_es_module, not load_main_es_module)
                        │               6. run_event_loop() — component renders to string
                        │               7. execute_script: render call → renderToString()
                        │               8. op_rwe_store_result(html) → thread-local RENDER_RESULT
                        │               Returns SsrResult { html, page_config }
                        │
                        │       RweModuleLoader: handles file:// only
                        │           transpiles .tsx/.ts/.jsx on load
                        │           "zeb"/"npm:*" imports → load error (stripped beforehand)
                        │
                        ├── Phase 2b — Client bootstrap (render.rs build_client_module):
                        │       1. strip_zeb_client_imports — strip "zeb" imports from bundle
                        │       2. deno_worker::transpile_client — OXC TSX→JS
                        │          CLIENT_TRANSPILE_CACHE: LazyLock<Mutex<HashMap<u64, String>>>
                        │          up to 256 cached entries
                        │       3. replace all `npm:preact` → `https://esm.sh/preact@10.28.4`
                        │       4. emit inline JS: import hooks from esm.sh, assign to globalThis
                        │       5. install SPA navigator (rweNavigate + NProgress bar)
                        │       6. base64-encode user bundle
                        │       7. bootstrap: hydrate(h(__RweRoot, null, h(__UserPage, __input)),
                        │                           document.getElementById('__rwe_root'))
                        │
                        └── Assemble HTML (render.rs build_document_shell):
                                body_content =
                                  <div id="__rwe_root">{SSR html}</div>
                                  <script type="application/json" id="__rwe_payload">{vars}</script>

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
                                  <script type="module">{client_bootstrap}</script>
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

### 6c. RWE Globals — Available in Templates Without Import

These are installed as `globalThis.*` by both SSR and client bootstrap.
Templates use them directly; `import { ... } from "zeb"` is stripped at compile time.

> **`Tool.*` is the Zebflow JS standard library** (`src/language/runtime/tool_init.js`).
> It is available in **every JS execution context**: RWE SSR, RWE client hydration, and `n.script`
> pipeline nodes. Four namespaces: `Tool.time`, `Tool.arr`, `Tool.stat`, `Tool.geo`.

| Global | SSR source (preact_ssr_init.js) | Client source (esm.sh) |
|--------|---------------------------------|------------------------|
| `h` | Preact h (stub) | `preact` |
| `Fragment` | Preact Fragment (stub) | `preact` |
| `React` | compat alias | `preact/compat` |
| `useState` | noop stub (SSR) | `preact/hooks` |
| `useEffect` | noop stub (SSR) | `preact/hooks` |
| `useRef` | noop stub (SSR) | `preact/hooks` |
| `useMemo` | noop stub (SSR) | `preact/hooks` |
| `useCallback` | noop stub (SSR) | `preact/hooks` |
| `useContext` | noop stub (SSR) | `preact/hooks` |
| `useReducer` | noop stub (SSR) | `preact/hooks` |
| `useId` | noop stub (SSR) | `preact/hooks` |
| `useLayoutEffect` | noop stub (SSR) | `preact/hooks` |
| `forwardRef` | passthrough (SSR) | `preact/compat` |
| `memo` | passthrough (SSR) | `preact/compat` |
| `createContext` | Preact createContext | `preact` |
| `usePageState` | RWE custom (SSR init) | RWE custom (client) |
| `cx` | classnames shim | classnames shim |
| `Link` | RWE SPA link (SSR noop) | RWE SPA link (client) |
| `useNavigate` | — (client only) | RWE SPA nav |

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
        ├── build_pipeline_graph(): detect mode
        │       body contains "[" and "] ->" → graph mode
        │       else                         → pipe mode
        │
        ├── PIPE MODE (split_pipe_segments):
        │       Quote-aware split on "|" — respects single- and double-quoted strings.
        │       A bare "|" splits into a new node; "|" inside a JS string literal is ignored.
        │       Each segment: "node_kind --flag val ... [-- raw_text]"
        │
        ├── GRAPH MODE (build_graph_mode):
        │       Each line is either:
        │         Node declaration:  [label] node_kind --flag val ...
        │         Edge declaration:  [from]:out_pin -> [to]:in_pin
        │                           (pin names optional; default: "out" → "in")
        │       entry_nodes = labels with no incoming edges
        │       Builds PipelineGraph { id, entry_nodes, nodes, edges }
        │
        ├── expand_kind():       "trigger.webhook" → "n.trigger.webhook"
        ├── parse_node_config(): --flag → config JSON key
        │       Note: --template sets BOTH "template_id" AND "template_path" keys
        │       Global flags (all nodes):
        │         --path         → "path"
        │         --method       → "method"
        │         --credential   → "credential_id"
        │         --template     → "template_id" + "template_path"  (both set)
        │         --route        → "route"
        │         --room         → "room"
        │         --event        → "event"
        │         --op           → "op"
        │         --to           → "to"
        │         -- SQL text    → "sql"
        │
        │       Node-specific dsl_flags (declared per-node in definition()):
        │         n.trigger.webhook:
        │           --path           → "path"           (required)
        │           --method         → "method"
        │           --auth-type      → "auth_type"      (none|jwt|hmac|api_key)
        │           --auth-credential→ "auth_credential"
        │         n.auth.token.create:
        │           --credential     → "credential_id"  (required)
        │           --expires-in     → "expires_in"
        │           --set-cookie     → "set_cookie"
        │           --cookie-name    → "cookie_name"
        │         n.sjtable.query:
        │           --table          → "table"
        │           --connection     → "connection"
        │           --op             → "operation"      (query|upsert)
        │           --operation      → "operation"
        │           --id-path        → "row_id_path"
        │           --limit          → "limit"
        │           --where-field    → "where_field"
        │           --where-value    → "where_value_expr"
        │         n.sjtable.mutate:
        │           --table          → "table"
        │           --op             → "operation"      (delete)
        │           --operation      → "operation"
        │           --id-path        → "row_id_path"
        │
        └── builds PipelineGraph { nodes, edges }
        │
        ├──→ register_pipeline → saved to repo/pipelines/*.zf.json  (status: draft)
        │       activate_pipeline → PipelineRuntimeService.activate() → goes live
        │
        └──→ run_ephemeral → BasicPipelineEngine.execute() directly (not saved)

DSL Verb signatures (executor.rs cmd_* methods):

  register <file_rel_path> [--title <t>] [--as-json] | <pipe body>
        file_rel_path is the canonical path, e.g. pipelines/api/my-hook
        normalize_pipeline_file_rel_path() ensures pipelines/ prefix + .zf.json suffix
        creates parent dirs; upsert_pipeline_definition(file_rel_path, ...)

  activate   <file_rel_path>
  deactivate <file_rel_path>
  execute    <file_rel_path> [--input <json>]
  describe   pipeline <file_rel_path>
  patch      pipeline <file_rel_path> node <node_id> [--flag val ...] [-- body]

All verbs resolve the pipeline via get_pipeline_meta_by_file_id(file_rel_path).
There is NO lookup by name alone — file_rel_path is the only key. See §21.
```

---

## 9. Data Layers

```
Layer 1 — Platform Catalog
  data/platform/catalog/  (SekejapDB)
      users, auth sessions, MCP sessions, credentials, pipeline hits, invocation log
      pipeline_meta collection — one node per pipeline per project
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

### 9a. pipeline_meta Sekejap Key Scheme

```
_id format:  "pipeline_meta/{owner}/{project}/{file_rel_path_slug}"

file_rel_path_slug:
  "pipelines/api/foo.zf.json"  →  "pipelines-api-foo-zf-json"
  All non-alphanumeric non(-/_) chars replaced with "-"; runs of "-" collapsed.

Example:
  "pipeline_meta/superadmin/default/pipelines-api-my-webhook-zf-json"

Stored fields:
  _id, _collection, owner, project, name, title, file_rel_path,
  description, trigger_kind, hash, active_hash, activated_at,
  created_at, updated_at

Derived on read (not stored):
  virtual_path  ← virtual_path_from_file_rel_path(file_rel_path)

put_pipeline_meta and delete_pipeline_meta use the same pipeline_slug() function —
key on write == key on delete, always.
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
  ├── SimpleTableService       SjTable CRUD via SekejapDB (query, upsert, delete_row)
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
Valid import specifiers — exhaustive, no exceptions:

  "zeb"     import { useState, useEffect, useRef, useMemo, useCallback, useContext,
                     useReducer, useId, useLayoutEffect, usePageState, useNavigate,
                     cx, Link, forwardRef, memo, createContext, Fragment } from "zeb"
              → STRIPPED at compile time; all are globalThis.* in SSR and client

  "zeb/*"   import { ThreeCanvas } from "zeb/threejs"
              → rewritten to versioned bundle URL at compile time
              → library must be installed in project (see §13)

  "@/"      import Button from "@/components/ui/button"
              → resolved to absolute filesystem path at compile time

Everything else → compile error RWE_IMPORT_NOT_ALLOWED or RWE_IMPORT_ZEB_ONLY.
No npm:, no node:, no jsr:, no relative paths. No exceptions.

External scripts (CDN / third-party):
  Loaded via load_scripts config on n.web.render node.
  Injected as <script> tags → populate window.x.{slug} namespace.
  Accessed in TSX as plain property access: x.lodash.map(...)
  No import statement needed or allowed.

NEVER:
  ✗ import { render } from "npm:preact" — never call render() manually
  ✗ relative imports (../../components/...) — use @/ alias
  ✗ import anything from outside "zeb", "zeb/*", "@/"
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
  template file 'pages/x' not found           resolve_template_entry() requires explicit .tsx
                                              extension. PUT /templates/file is upsert —
                                              write_template_file() calls create_dir_all() first.

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

UI form fields appear white in dark mode       Never use dark: classes — custom Tailwind
                                              compiler does NOT support the dark: variant.
                                              Use bg-[var(--zf-ui-bg)] etc. instead.
                                              See §14 for the full token system.

Node dialog shows wrong/missing fields        Fields come from Rust definition() via
                                              /docs/node API. Check NodeFieldDef vec in
                                              the node's definition() return value.
                                              Restart server to pick up changes.

PLATFORM_PIPELINE_MISSING on pipeline load     Physical file at repo/pipelines/… is absent.
                                              delete_pipeline removes both the file and the
                                              catalog row. If file is missing but catalog row
                                              exists, delete via API to clean the catalog row.
```

---

## 13. Library System (`zeb/*`)

### 13a. Overview

`zeb/*` libraries are **frontend JavaScript libraries** used exclusively inside TSX templates.
They are NOT pipeline nodes. Examples: `zeb/threejs`, `zeb/d3`, `zeb/codemirror`.

Each library is a **single self-contained ESM bundle** per version — the raw npm library
(e.g. Three.js r183) + Preact wrapper components (e.g. `ThreeCanvas`) + utility functions
(e.g. `createSceneRuntime`), all pre-compiled into one minified `.mjs` file.

Preact is **external** — not bundled inside libraries. Every RWE page already has Preact
loaded as a global. Libraries assume `preact` and `preact/hooks` are available.

```
User writes:    import { ThreeCanvas, BoxGeometry } from "zeb/threejs"
                import { scaleLinear, BarChart } from "zeb/d3"
                import { useState } from "zeb"

RWE compiler    "zeb" → stripped (hooks are globalThis globals)
transforms:     "zeb/threejs" → lookup zeb.lock → rewrite to versioned bundle URL
                "zeb/d3" → lookup zeb.lock → rewrite to versioned bundle URL

Browser loads:  /p/{owner}/{project}/lib/zeb/threejs/r183/bundle.min.mjs  (once, cached immutably)
```

### 13b. User Flow

```
Step 1 — Create project

    User: /home → Create "my-3d-app" → Submit
    Platform: project folder created, empty zeb.lock, empty .libraries/ dir
    User lands on project studio. No libraries installed.

Step 2 — Settings → Libraries tab

    User sees card grid of all available zeb/* libraries.
    Each card shows: name, description, status (Not installed / Installed: r183).
    Each card is a LINK — clicking navigates to the library detail page.

Step 3 — Library detail page  (/projects/{owner}/{project}/settings/libraries/zeb/threejs)

    Header: zeb/threejs — "3D rendering, scene management, canvas component"
    Current status: Not installed

    Version selector shows 3 categories of versions:

    ┌─────────────────────────────────────────────────────────────┐
    │  Available Versions                                         │
    │                                                             │
    │  ● r183  ·  bundled (offline)   ·  600KB   →  [Install]    │
    │                                                             │
    │  ○ r190  ·  online repo         ·  620KB   →  [Download]   │
    │  ○ r188  ·  online repo         ·  615KB   →  [Download]   │
    │                                                             │
    │  ○ r182  ·  previously cached   ·  598KB   →  [Install]    │
    └─────────────────────────────────────────────────────────────┘

    Bundled:              embedded in binary, instant install, always available offline
    Online:               from registry, requires download, shows size estimate
    Previously cached:    downloaded before, cached on disk, instant re-install

Step 4 — Install r183 (bundled/offline)

    User clicks [Install] on r183.
    Platform:
      1. Extracts r183/bundle.min.mjs from binary
         → {project}/.libraries/zeb/threejs/r183/bundle.min.mjs
      2. Updates zeb.lock:
         { "zeb/threejs": { "version": "r183", "integrity": "sha256:..." } }
      3. Updates zebflow.json:
         rwe.libraries["zeb/threejs"] = { version: "r183", source: "offline" }

    Card status changes to: Installed — r183 · offline
    Library is ready to use in templates immediately.

Step 5 — Install r190 (online)

    User clicks [Download] on r190.
    Progress bar shown.
    Platform:
      1. Downloads from registry URL → verifies sha256 integrity
      2. Writes to {project}/.libraries/zeb/threejs/r190/bundle.min.mjs
      3. Caches to shared download cache (other projects can reuse)
      4. Updates zeb.lock + zebflow.json
    Status changes to: Installed — r190 · online

Step 6 — Switch version

    User goes back to library detail page.
    Currently installed: r190. User clicks [Install] on r183 instead.
    Platform swaps: updates zeb.lock to point to r183.
    Both versions stay on disk in .libraries/. No re-download.
    Next page compile picks up r183 from the new URL.

Step 7 — Use in template

    import { ThreeCanvas, Scene, BoxGeometry, createSceneRuntime } from "zeb/threejs"

    All symbols — raw Three.js classes, wrapper Preact components, utility functions
    — come from one import specifier. One bundle. One network request.

Step 8 — Compile + serve

    RWE compiler sees: import { ... } from "zeb/threejs"
    Reads project zeb.lock → version = "r183"
    Rewrites import to: /p/{owner}/{project}/lib/zeb/threejs/r183/bundle.min.mjs
    Browser fetches bundle once (~180KB gzip'd). Cached with immutable headers.
    Version upgrade → URL changes → automatic cache bust.

Step 9 — Version upgrade

    We ship r190 as new embedded version (or user downloads it).
    User goes to library detail page → clicks [Install] on r190.
    zeb.lock updated. Next page compile uses r190 URL.
    Old browser cache misses on new URL → fetches r190 once → done.
    No template changes needed. Same import, newer library.

Step 10 — Offline scenario

    User has no internet. Library detail page shows:
    Bundled versions: available, installable
    Online versions: greyed out — "Requires internet connection"
    Previously cached versions: available, installable
    Everything on disk works. No surprises.

Step 11 — Uninstall

    Library detail page has [Uninstall] button.
    Removes from zeb.lock + zebflow.json. Removes active version from .libraries/.
    Downloaded cache stays (for re-install later).
    If templates still import "zeb/threejs", compiler throws clear error:
    "Library zeb/threejs is not installed. Enable it in Settings → Libraries."
```

### 13c. Bundle Artifact Structure

```
libraries/zeb/threejs/
  manifest.json                  ← symbol registry + all available versions
  r183/
    bundle.min.mjs               ← embedded in binary (offline default)
  r190/                          ← would be downloaded on demand
    bundle.min.mjs

manifest.json:
{
  "name": "zeb/threejs",
  "description": "Three.js — 3D rendering, scene management, canvas component",
  "exports": [
    "Scene", "PerspectiveCamera", "WebGLRenderer", "BoxGeometry",
    "MeshStandardMaterial", "Mesh", "DirectionalLight", "Color",
    "ThreeCanvas", "createSceneRuntime", "mountThreeScene", "..."
  ],
  "versions": {
    "r183": {
      "entry": "r183/bundle.min.mjs",
      "source": "offline",
      "package_version": "0.183.2",
      "size_bytes": 620000,
      "integrity": "sha256:abc123..."
    },
    "r190": {
      "entry": "r190/bundle.min.mjs",
      "source": "online",
      "package_version": "0.190.0",
      "size_bytes": 640000,
      "registry_url": "https://github.com/zebflow/libraries/releases/download/zeb-threejs-r190/bundle.min.mjs",
      "integrity": "sha256:def456..."
    }
  }
}
```

### 13d. Build Chain (how bundles are produced)

```
npm package (three@0.183.2)
+ wrapper utilities (createSceneRuntime, mountThreeScene)
+ Preact components (ThreeCanvas.tsx — pre-compiled to JS)
        │
        ▼ esbuild
            --bundle
            --minify
            --format=esm
            --platform=browser
            --external:preact
            --external:preact/hooks
        │
        ▼
bundle.min.mjs  (~600KB, self-contained except Preact)
        │
        ▼ include_bytes! in embedded.rs
        │
        ▼ embedded in binary
```

### 13e. Project File Layout

```
{project}/
  repo/
    zebflow.json              ← rwe.libraries: { "zeb/threejs": { version, source } }
    zeb.lock                  ← pinned versions + integrity hashes (git-tracked)
  .libraries/                 ← gitignored, reproduced from zeb.lock
    zeb/
      threejs/
        r183/
          bundle.min.mjs      ← extracted from binary or downloaded
      d3/
        r7.9/
          bundle.min.mjs
```

### 13f. Platform Services

```
LibraryService           in-memory registry of embedded library manifests
                         parsed from PLATFORM_LIBRARY_ASSETS at startup
                         methods: list(), get(name)

ZebLockService           reads/writes repo/zeb.lock per project
                         methods: read, write, add_entry, remove_entry

ZebflowJsonService       rwe.libraries section in zebflow.json
                         methods: get_rwe_libraries, enable_rwe_library, disable_rwe_library

ProjectService           on create_or_update_project:
                         writes empty zeb.lock, creates .libraries/ dir
                         does NOT seed any default pipelines — projects start with zero pipelines
                         users create their own pipelines from scratch or from templates
```

### 13g. API Routes

```
GET    /projects/{owner}/{project}/settings/libraries
         Library list page (cards linking to detail pages)

GET    /projects/{owner}/{project}/settings/libraries/{library_name}
         Library detail page with version selector

GET    /api/projects/{owner}/{project}/rwe/libraries
         JSON: all libraries with status + available versions

POST   /api/projects/{owner}/{project}/rwe/libraries/install
         Body: { name, version }
         Extracts/downloads bundle → .libraries/ → updates zeb.lock + zebflow.json

DELETE /api/projects/{owner}/{project}/rwe/libraries/uninstall
         Query: ?name=zeb/threejs
         Removes from zeb.lock + zebflow.json + .libraries/

GET    /p/{owner}/{project}/lib/{*path}
         Static file serving from project .libraries/ directory
         Cache-Control: public, max-age=31536000, immutable
```

### 13h. RWE Compiler Integration

```
In compiler.rs — during import resolution:

1. Encounter: import { ThreeCanvas, BoxGeometry } from "zeb/threejs"
2. Read project's zeb.lock → find "zeb/threejs" → version "r183"
3. Rewrite specifier:
     from:  "zeb/threejs"
     to:    "/p/{owner}/{project}/lib/zeb/threejs/r183/bundle.min.mjs"
4. Import remains in compiled output (NOT stripped like "zeb")
5. Browser resolves the rewritten URL at runtime

If "zeb/threejs" is NOT in zeb.lock → compile error:
  "Library zeb/threejs is not installed. Enable it in Settings → Libraries."

Optional: validate imported symbols against manifest.json exports list.
  Unknown symbol → compile warning:
  "ThreeCanvass is not exported by zeb/threejs. Did you mean: ThreeCanvas?"

---

## Library Bundle Build Recipes

`zeb/*` library bundles are **offline, self-contained ESM files** — no CDN fetches at runtime.
All npm dependencies are bundled inline using esbuild.  Build in `/tmp` first, then move.

### zeb/prosemirror

```sh
mkdir -p /tmp/zeb-pm-build && cd /tmp/zeb-pm-build

# package.json (already in libraries/zeb/prosemirror/0.1/ — copy or recreate):
cat > package.json <<'EOF'
{
  "name": "zeb-prosemirror-build", "version": "1.0.0", "private": true, "type": "module",
  "dependencies": {
    "prosemirror-commands": "^1.5.0", "prosemirror-gapcursor": "^1.3.0",
    "prosemirror-history": "^1.4.0",  "prosemirror-keymap": "^1.2.0",
    "prosemirror-model": "^1.23.0",   "prosemirror-schema-basic": "^1.2.0",
    "prosemirror-schema-list": "^1.4.0", "prosemirror-state": "^1.4.0",
    "prosemirror-view": "^1.30.0"
  },
  "devDependencies": { "esbuild": "^0.25.0" }
}
EOF

# Copy entry.mjs from the source repo:
cp /path/to/zebflow/libraries/zeb/prosemirror/0.1/runtime/entry.mjs .
# OR keep entry.mjs already in /tmp/zeb-pm-build/ from a previous build.

npm install
node_modules/.bin/esbuild entry.mjs --bundle --format=esm --minify \
  --outfile=prosemirror.bundle.mjs

# Move the result:
cp prosemirror.bundle.mjs \
  /path/to/zebflow/libraries/zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs
```

Output size: ~237KB minified (all PM packages + application code inline).

The `entry.mjs` source lives at `libraries/zeb/prosemirror/0.1/runtime/` alongside the bundle.
It contains the full application logic with `import * as _pmX from "prosemirror-X"` at the top.
esbuild resolves those imports from `node_modules/` and inlines everything into the single output file.

### zeb/threejs (same pattern)

```sh
# Three.js bundle uses the same offline approach.
# Source: libraries/zeb/threejs/r183/bundle.min.mjs (pre-built, vendored directly).
# No build script needed — the bundle is checked into the repo as-is.
```

### General rule for all zeb/* libraries

1. Source entry file lives at `libraries/zeb/{name}/{version}/runtime/entry.mjs`
2. Output bundle at `libraries/zeb/{name}/{version}/runtime/{name}.bundle.mjs`
3. Build uses: `esbuild entry.mjs --bundle --format=esm --minify --outfile=bundle.mjs`
4. The bundle is committed to the repo — no runtime npm installs on the server
5. `embedded.rs` embeds the bundle via `include_bytes!` → served at `/assets/libraries/...`
```

---

## 14. Platform UI Theming

### 14a. Approach — CSS Variable Semantic Tokens

Platform UI components (`components/ui/`) use **CSS variable semantic tokens** for theming.
No Tailwind `dark:` classes are used. This is the same approach used by Atlassian DS, IBM Carbon,
GitHub Primer: define semantic tokens that switch under a `data-theme` attribute.

```
:root                              ← Light defaults (login, home, non-studio pages)
.project-studio-frame              ← Dark overrides (studio is dark by default)
.project-studio-frame[data-theme="light"]  ← Restore light (studio light mode toggle)
```

### 14b. Token Set

Defined in `src/platform/web/templates/styles/main.css`:

```css
/* 8 semantic UI tokens */
--zf-ui-bg             surface background      #ffffff / #020617
--zf-ui-bg-subtle      card headers, code editor headers   #f8fafc / #0f172a
--zf-ui-bg-muted       tabs list, hover states #f1f5f9 / #1e293b
--zf-ui-border         default borders         #e2e8f0 / #1e293b
--zf-ui-border-subtle  card header border      #f1f5f9 / #1e293b
--zf-ui-text           primary text            #020617 / #f8fafc
--zf-ui-text-soft      secondary text          #475569 / #94a3b8
--zf-ui-text-muted     labels, placeholders    #64748b / #94a3b8
```

`--studio-*` vars (also on `.project-studio-frame`) remain for structural layout
(sidebar, panel backgrounds, etc.) and are separate from the UI component tokens.

### 14c. Usage in UI Components

```tsx
// CORRECT — theme-aware
<input className="bg-[var(--zf-ui-bg)] border-[var(--zf-ui-border)] text-[var(--zf-ui-text)]" />

// WRONG — hardcoded, breaks in dark mode
<input className="bg-white border-slate-200 dark:bg-slate-950 dark:border-slate-800" />
```

The custom Tailwind compiler (`src/rwe/processors/tailwind/compiler.rs`) supports
arbitrary CSS variable syntax: `bg-[var(--zf-ui-bg)]` compiles to
`background-color: var(--zf-ui-bg)` correctly via `color_value()` → `arbitrary_value()`.

Supported positional utilities include `inset-y-*` (`top+bottom`) and `inset-x-*` (`left+right`).

The `dark:` variant is **NOT supported** by the custom compiler — it returns `None`
for unknown variants. Never use `dark:` classes in platform templates.

Placeholder colors are handled via direct CSS (compiler doesn't support `placeholder:` variant):
```css
.project-studio-frame input::placeholder,
.project-studio-frame textarea::placeholder {
  color: var(--zf-ui-text-muted);
}
```

### 14d. Theme Toggle

`project-studio-shell.tsx` holds `const [theme, setTheme] = useState("dark")`.
The toggle button sets `data-theme` on `.project-studio-frame`. CSS variables cascade
automatically — no JS needed to restyle individual components.

```tsx
<div className="project-studio-frame" data-theme={theme}>
  {/* all studio content — tokens resolve from nearest ancestor */}
</div>
```

### 14e. UI Component Size Baseline (shadcn)

Interactive form controls match shadcn/ui defaults:

```
Input   h-9 py-1 px-3 text-sm   (components/ui/input.tsx)
Select  h-9 py-1 px-3 text-sm   (components/ui/select.tsx)
Button  md: h-9 px-4  sm: h-8 px-3  xs: h-7 px-2.5  lg: h-10 px-6
```

### 14f. DropdownMenu Component

`components/ui/dropdown-menu.tsx` — state-driven dropdown, no native `<details>/<summary>`.

```
API:
  <DropdownMenu trigger={<Button size="sm" variant="outline">+ New</Button>} align="right">
    <DropdownMenuItem label="Option" onClick={...} />
  </DropdownMenu>

Behaviour:
  - open/close via useState
  - outside-click close via useEffect + document mousedown listener
  - item-click auto-close via onClick bubbling on content wrapper
  - align: "left" (default) | "right" | "center"
```

`DropdownMenuContent` remains as a standalone styled panel used directly in
`project-studio-shell.tsx` for the Repo and MCP panels (which manage their own open state).

### 14g. Alert Variant Colors

Alert uses alpha-based semantic colors that work on both dark and light backgrounds
without per-theme overrides:

```tsx
error:   "border-red-500/30 bg-red-500/10 text-red-500"
warning: "border-yellow-500/30 bg-yellow-500/10 text-yellow-500"
success: "border-green-500/30 bg-green-500/10 text-green-500"
info:    "border-blue-500/30 bg-blue-500/10 text-blue-500"
```

---

## 15. Pipeline Node Field Definitions (Server-Driven UI)

### 15a. Overview

Pipeline node configuration dialogs are **server-driven** — each node declares its own
form fields in Rust via `definition()`, which returns a `NodeDefinition` containing
`fields: Vec<NodeFieldDef>`. The frontend never hardcodes per-node field logic.

```
GET /api/projects/{owner}/{project}/pipeline/docs/node
        → Vec<NodeContractItem>  each with  fields: Vec<NodeFieldDef>
        → catalog stored in PipelineEditor JS memory
        → on "E" click: look up fields from catalog, pass to <NodeForm>
```

### 15b. Rust Structs

Defined in `src/pipeline/model.rs` and `src/pipeline/nodes/interface.rs`:

```rust
NodeFieldDef {
    name: String,          // config key
    label: String,
    field_type: NodeFieldType,  // text | textarea | code_editor | select | datalist |
                                //   method_buttons | copy_url | checkbox | section
    help: Option<String>,
    placeholder: Option<String>,
    readonly: bool,
    rows: Option<u32>,
    language: Option<String>,   // code_editor: "javascript" | "sql" | "json"
    options: Vec<SelectOptionDef>,
    data_source: Option<NodeFieldDataSource>,  // credentials_postgres | credentials_jwt
                                               //   | templates_pages
    default_value: Option<Value>,
    sidebar: Vec<SidebarSection>,  // code_editor sidebar: collapsible reference panels
    span: Option<String>,          // "full" | "half" — overrides default grid span
}

// Hierarchical layout tree for the config dialog.
// Serializes as untagged JSON: "field_name" | {"row":[...]} | {"col":[...]}
// Empty layout → fall back to flat 2-column grid using fields order.
LayoutItem {
    Field(String),                  // references a NodeFieldDef by name
    Row { row: Vec<LayoutItem> },   // horizontal group, equal-width flex columns
    Col { col: Vec<LayoutItem> },   // vertical stack inside a row cell
}
```

`NodeDefinition` and `NodeContractItem` both carry:
- `fields: Vec<NodeFieldDef>`
- `layout: Vec<LayoutItem>` — skip_serializing_if empty; all 19 built-in nodes declare layout

### 15c. Frontend Rendering Layer

`src/platform/web/templates/components/nodes/`:

```
node-form.tsx              enrichFields() → if layout present: <NodeLayout>
                                          → if no layout: flat 2-col grid fallback
node-layout.tsx            recursive layout renderer — Row/Col/Field tree → JSX
node-field.tsx             dispatcher by field.type
node-field-text.tsx        <Field><Input>       half-width default
node-field-textarea.tsx    <Field><Textarea>    full-width
node-field-code-editor.tsx <CodeEditor> + collapsible sidebar  full-width
node-field-select.tsx      <Field><Select>      half-width
node-field-datalist.tsx    <Field><Input list>  full-width
node-field-method-buttons.tsx  HTTP method toggle row  full-width
node-field-copy-url.tsx    read-only input + Copy button  full-width
node-field-checkbox.tsx    <Checkbox>           half-width
node-field-section.tsx     <Separator> heading  full-width
```

Grid span rules (fallback only, overridable by `field.span`):
- **full**: `code_editor`, `textarea`, `datalist`, `method_buttons`, `copy_url`, `section`
- **half**: `text`, `select`, `checkbox`

`enrichFields()` in `node-form.tsx` resolves live values:
- `data_source: credentials_postgres` → `dataState.pgCredentials` list → `<Select>` options
- `data_source: credentials_jwt` → `dataState.jwtCredentials`
- `data_source: templates_pages` → `dataState.pageTemplates`
- `type: copy_url` → builds webhook public URL from `window.location.origin + config.path`

### 15d. OXC Parser — `localize_exports`

`src/rwe/core/compiler.rs` — `localize_exports()` strips `export` keywords from inlined
component files so they become local declarations (no duplicate module exports in the flat bundle).

Uses **OXC AST byte-span surgery**:
- Walks `parsed.program.body` for `ExportNamedDeclaration` and `ExportDefaultDeclaration` nodes
- TS-only declarations (`TSTypeAliasDeclaration`, `TSInterfaceDeclaration`) → entire declaration removed
- Value declarations (`FunctionDeclaration`, `ClassDeclaration`, `VariableDeclaration`) → only the `export` prefix bytes removed, declaration body kept
- Collects byte-range operations, applies them in one pass over the source string
- Falls back to returning source unchanged if OXC panics (`parsed.panicked`)

### 15e. Adding a New Node

1. Create `src/pipeline/nodes/basic/<name>.rs` — implement `NodeHandler` trait (`execute_async`)
2. Declare `pub fn definition() -> NodeDefinition` with `kind`, `fields`, `layout`, `dsl_flags`
3. Declare `pub mod <name>;` in `src/pipeline/nodes/basic/mod.rs` and add `definition()` to `builtin_node_definitions()`
4. Add dispatch arm in `BasicPipelineEngine::build_node()` — deserialize config, construct `NodeDispatch::<Variant>(node)`
5. Add `<Variant>(Node)` to the `NodeDispatch` enum and a match arm in `execute_async`
6. If DSL flag names differ from config struct field names, declare explicit `dsl_flags: vec![DslFlag { flag, config_key, .. }]` — the default auto-transform maps `--flag-name` → `flag_name` only for exact matches
7. No frontend changes — `NodeForm` renders `NodeFieldDef[]` generically from the `/docs/node` API

---

## 16. `n.sjtable.mutate` — SjTable Row Mutations

Separate from `n.sjtable.query`. Handles write operations on SjTable rows that are not reads.

### 16a. Supported Operations

| operation | behaviour |
|---|---|
| `delete` | Removes the row entirely from the SekejapDB collection. Calls `db.nodes().remove(&slug)`. |

### 16b. Config Fields

| Field | DSL Flag(s) | Description |
|---|---|---|
| `table` | `--table` | SjTable name (must match a configured SjTable connection) |
| `operation` | `--op`, `--operation` | `"delete"` |
| `row_id_path` | `--id-path` | JSON pointer into the input payload to extract the row ID |
| `row_id_expr` | — | Alternative JS expression for row ID (evaluated if row_id_path empty) |

### 16c. Output

```json
{ "deleted": true, "row_id": "<resolved-id>" }
```

HTTP callers should check the HTTP response status (`r.ok`) not a body `ok` field.

### 16d. Example DSL (Graph Mode)

```
| trigger.webhook --method DELETE --path /admin/posts/:id
| sjtable.mutate --table posts --op delete --id-path params.id
```

### 16e. Files

```
src/pipeline/nodes/basic/sjtable_mutate.rs     node implementation
src/platform/services/simple_table.rs         delete_row() method
src/pipeline/nodes/basic/mod.rs               sjtable_mutate registered
src/pipeline/engines/basic.rs                 NodeDispatch::SimpleTableMutate arm
```

---

## 17. Webhook Response Convention

```
Response selection (in order of priority):

1. output.value["html"] exists
        → inject Tailwind CSS → externalize JS → Html(html).into_response()

2. output.value["_status"] exists
        → status = _status value  (u16)
        → body = output.value without _status + without _set_cookie
        → if status >= 400: use dispatch_weberror()
        → if status < 400:  (status, Json(body))

3. _set_cookie extraction (applies to paths 2+3):
        → if output.value["_set_cookie"] exists, build Set-Cookie header from it
        → strip _set_cookie from response body

4. else (default)
        → Json(output.value_without_set_cookie).into_response()   [HTTP 200]
        → direct pipeline output passthrough — no extra envelope

   IMPORTANT: Do NOT use `_status: 200` workarounds to return plain JSON.
   The fallback path already returns plain JSON. Use _status only when you need
   a non-200 HTTP status code.
```

---

## 18. Template File API

```
PUT  /api/projects/{owner}/{project}/templates/file
        Body: { path: "pages/blog.tsx", content: "export default ..." }
        Behaviour: UPSERT — creates missing intermediate directories automatically.
                   write_template_file() calls fs::create_dir_all(parent) before fs::write.

GET  /api/projects/{owner}/{project}/templates/pages
        Query: ?path=  (optional prefix filter, e.g. "/pages" or "/components/ui")
        Response:
          { "ok": true, "path": "/", "items": [
              { "name": "home.tsx", "rel_path": "pages/home.tsx", "file_kind": "page" },
              { "name": "button.tsx", "rel_path": "components/ui/button.tsx",
                "file_kind": "component" }
          ]}
        Returns: all .tsx files in the project template root, recursively.
        file_kind:  "page" if under pages/   "component" otherwise.
        Used by: n.web.render dialog, MCP tools, REST clients.
```

---

## 19. MCP Layer

### 19a. Source

```
src/platform/mcp/handler.rs     ZebflowMcpHandler — all 25 tool implementations
src/platform/model.rs           mcp_tool_capability() — tool slug → ProjectCapability mapping
src/platform/web/mod.rs         build_mcp_service() — axum router, middleware, service binding
```

### 19b. Request Flow

```
POST /api/projects/{owner}/{project}/mcp
        │
        ▼ axum middleware (from_fn in build_mcp_service)
        │   reads Authorization: Bearer <token> header
        │   platform.mcp_sessions.lookup(token) → Option<McpSession>
        │   if found: req.extensions_mut().insert(session)
        │
        ▼ StreamableHttpService (rmcp)
        │   stateful_mode = false  (stateless per-request)
        │   json_response = true
        │   sse_keep_alive = 30s
        │   spawns ZebflowMcpHandler per request
        │
        ▼ ZebflowMcpHandler::tool_router() (generated by #[tool_router] macro)
        │   routes tool name → method
        │
        ▼ individual #[tool] method
            Extension(parts): Extension<http::request::Parts>
                → parts.extensions.get::<McpSession>()  ← injected by middleware
                → check_tool_capability(&session, "tool_name")
                    → mcp_tool_capability("tool_name") → ProjectCapability variant
                    → platform.authz.ensure_project_capability(subject, owner, project, cap)
                    → McpError if denied
            Parameters(params): Parameters<XxxParams>
                → rmcp deserializes JSON tool arguments into typed struct
            → execute business logic using platform.* services
            → CallToolResult::success(vec![Content::text(json)])
```

### 19c. Capability Mapping

`mcp_tool_capability(tool_name)` in `src/platform/model.rs` maps every tool slug to a `ProjectCapability`.
Every tool call goes through `check_tool_capability` — no tool bypasses authz.

```
PipelinesRead     → list_pipelines, get_pipeline, describe_pipeline, list_connections,
                    describe_connection, list_credentials, list_project_docs, read_project_doc,
                    list_agent_docs, read_agent_doc, list_skills, read_skill, list_templates,
                    get_template
PipelinesWrite    → register_pipeline, patch_pipeline, activate_pipeline, deactivate_pipeline,
                    write_doc, write_agent_doc, git_command
PipelinesExecute  → execute_pipeline, run_ephemeral
TemplatesCreate   → create_template
TemplatesWrite    → write_template
```

### 19f. MCP Tool Param Structs

All pipeline-targeting MCP tools use `file_rel_path` as their sole pipeline identifier.
There is no `name` or `virtual_path` parameter anywhere in the MCP layer.

```rust
PipelineRegisterParams  { file_rel_path: String, title: Option<String>, body: String }
PipelineGetParams       { file_rel_path: String }   // already was file_rel_path ✓
PipelineDescribeParams  { file_rel_path: String }
PipelinePatchParams     { file_rel_path: String, node_id: String,
                          flags: Option<String>, body: Option<String> }
PipelineActivateParams  { file_rel_path: String }
PipelineDeactivateParams{ file_rel_path: String }
PipelineExecuteParams   { file_rel_path: String, input: Option<String> }
PipelineRunParams       { body: String }            // ephemeral, no identity ✓
PipelineListParams      { prefix: Option<String> }  // filter by path prefix

DSL strings built by MCP handler:
  register pipelines/api/my-hook --title "My Hook" | trigger.webhook ...
  patch pipeline pipelines/api/my-hook node n0 --path /new-path
  activate pipelines/api/my-hook
  deactivate pipelines/api/my-hook
  execute pipelines/api/my-hook --input {}
  describe pipeline pipelines/api/my-hook
```

### 19d. Adding a New Tool

1. Add `#[derive(serde::Deserialize, JsonSchema)] struct XxxParams { ... }` for typed params
2. Add `#[tool(description = "...")] async fn xxx_tool(&self, Extension(parts): ..., Parameters(params): ...) -> Result<CallToolResult, McpError>` inside `#[tool_router] impl ZebflowMcpHandler`
3. Map the tool name to a `ProjectCapability` in `mcp_tool_capability()` in `model.rs`
4. Capability `ProjectCapability::PipelinesRead` or existing variant — add new variant if needed
5. No registration call needed — `#[tool_router]` macro collects all `#[tool]` methods at compile time

### 19e. Session Model

```rust
McpSession {
    token: String,              // Bearer token value
    owner: String,              // project owner
    project: String,            // project slug
    created_at: i64,            // unix timestamp
    auto_reset_seconds: Option<u64>,  // expiry window; None = never expires
    capabilities: Vec<String>,  // allowed tool slugs (or "*" for all)
}
```

Sessions are persisted to `platform/catalog/` SekejapDB (`mcp_session` collection).
`mcp_sessions.lookup(token)` loads session and lazily expires it if `auto_reset_seconds` elapsed.

---

## 20. Skills System

### 20a. Source

```
src/platform/skills/                 embedded markdown skill docs
    agent-core.md                    MCP server instructions (injected into get_info())
    zebflow-overview.md
    pipeline-dsl.md
    pipeline-nodes.md
    pipeline-authoring.md
    pipeline-dsl-rwe.md
    pipeline-dsl-web-auto.md
    rwe-templates.md
    project-operations.md
    full-project-workflow.md
    sekejapql.md
    api-reference.md
src/platform/skills/mod.rs           Skill struct, all_skills(), get_skill(), format_skills_for_system_prompt()
```

### 20b. Embedding

Skills are embedded at compile time via `include_str!`. No runtime file reads.

```rust
pub struct Skill {
    pub name: &'static str,    // slug used in read_skill MCP tool
    pub title: &'static str,   // display title
    pub content: &'static str, // full markdown text
}

pub fn all_skills() -> Vec<Skill> { ... }     // all skills in order
pub fn get_skill(name: &str) -> Option<Skill> // lookup by slug
```

### 20c. MCP Server Instructions

`ZebflowMcpHandler::get_info()` returns `ServerInfo { instructions: Some(...) }`.
The instructions string is the full content of `agent-core.md`.
MCP clients (Claude, Cursor, etc.) receive this on connect and use it as the agent's operating instructions.

### 20d. Assistant System Prompt

`format_skills_for_system_prompt()` concatenates all skill content into a single string.
This is prepended to the assistant's system prompt when the project assistant LLM is loaded
(`load_project_assistant_llm` in `src/platform/services/platform.rs`).

### 20e. Adding or Editing a Skill

- Edit the `.md` file in `src/platform/skills/`
- If adding a new skill: add a `Skill { name, title, content: include_str!(...) }` entry to `all_skills()` in `mod.rs`
- Rebuild — the new content is embedded in the binary
- No DB migration, no API change

---

## 21. Pipeline Identity Model

### 21a. Canonical Identifier

`file_rel_path` is the **only** stable, unique pipeline identifier.
It is the relative path of the pipeline JSON file under the project repo root.

```
pipelines/api/my-webhook.zf.json
pipelines/blog/list-posts.zf.json
pipelines/my-pipe.zf.json
```

Every service method, REST request body, DSL verb, and MCP tool param
accepts `file_rel_path` only. `virtual_path` and `name` are **never accepted as input**.

### 21b. Derived Fields

`name` and `virtual_path` are computed from `file_rel_path` on every read and are not stored.

```rust
// src/platform/services/project.rs

fn virtual_path_from_file_rel_path(file_rel_path: &str) -> String {
    // "pipelines/api/foo.zf.json" → "/api"
    // "pipelines/foo.zf.json"     → "/"
    let stripped = file_rel_path
        .trim_start_matches("pipelines/")
        .trim_start_matches('/');
    match stripped.rfind('/') {
        Some(pos) => format!("/{}", &stripped[..pos]),
        None => "/".to_string(),
    }
}

fn name_from_file_rel_path(file_rel_path: &str) -> String {
    // "pipelines/api/foo.zf.json" → "foo"
    Path::new(file_rel_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_end_matches(".zf"))
        .unwrap_or(file_rel_path)
        .to_string()
}
```

These are called in `list_pipeline_meta_rows` and `get_pipeline_meta_by_file_id`
after every DB read, so `PipelineMeta.virtual_path` and `.name` are always populated
for display purposes even though they are not persisted as the source of truth.

### 21c. Lookup by file_rel_path

```rust
// The only correct way to look up a pipeline by identity:
projects.get_pipeline_meta_by_file_id(owner, project, file_rel_path)

// NEVER:
rows.iter().find(|m| m.name == name)  // ← ambiguous: two pipelines can share a name
```

The old `get_pipeline_meta(owner, project, virtual_path, name)` function has been deleted.

### 21d. Normalization

`normalize_pipeline_file_rel_path(path)` ensures any user-supplied path is canonical:

```
"my-hook"                      → "pipelines/my-hook.zf.json"
"api/my-hook"                  → "pipelines/api/my-hook.zf.json"
"pipelines/api/my-hook"        → "pipelines/api/my-hook.zf.json"
"pipelines/api/my-hook.zf.json"→ "pipelines/api/my-hook.zf.json"  (already canonical)
```

### 21e. REST Request Bodies

```
POST /api/projects/{owner}/{project}/pipelines/definition
  { "file_rel_path": "pipelines/api/my-hook.zf.json",
    "title": "My Hook",
    "description": "",
    "trigger_kind": "webhook",
    "source": "{...pipeline JSON...}" }

POST /api/projects/{owner}/{project}/pipelines/activate
POST /api/projects/{owner}/{project}/pipelines/deactivate
  { "file_rel_path": "pipelines/api/my-hook.zf.json" }

POST /api/projects/{owner}/{project}/pipelines/execute
  { "file_rel_path": "pipelines/api/my-hook.zf.json",
    "trigger": "manual",
    "input": {} }

GET /api/projects/{owner}/{project}/pipelines/registry?prefix=api
GET /api/projects/{owner}/{project}/pipelines?prefix=api&recursive=true
```

### 21f. Why This Matters

Before this change, `pipeline_patch` (and activate/deactivate/execute via MCP) resolved
pipelines with `.find(|m| m.name == name)` — silently picking the alphabetically-first
match when two pipelines shared the same name under different paths (e.g. `api/posts` and
`admin/posts` both named `posts`). This was a real data-corruption risk.

Now every operation targets exactly one file. Two pipelines can have the same `name` slug
as long as they live in different directories — they are always unambiguous.
