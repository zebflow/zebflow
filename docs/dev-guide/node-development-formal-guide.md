# Node Development Formal Guide

## Node Structure

A Zebflow node has three layers:

1. **Kind contract** — `NodeDefinition`: kind, title, description, pins, schemas, DSL flags, UI fields, layout.
2. **Runtime implementation** — `NodeHandler`: config deserialization, execution, output payload, trace.
3. **Graph instance** — `PipelineNode`: slug, kind, per-instance config, position, edges.

## Node Engine Sources

Three formal sources. All produce the same `NodeDefinition` contract.

| Source | Namespace | What it is | Update model |
|--------|-----------|------------|--------------|
| Native | `n.*` | Compiled Rust module | Binary update |
| Composite | `n.c.*` | Function pipeline + manifest | Hot-update, project/hub artifact |
| WASM | `n.wasm.*` | Sandboxed binary module | Hot-update, signed package |

## Node Package Contract

Node package format is paramount. Every package source must pass the same
strict parser and validator before it can become available in the node catalog.
Invalid shape, missing definition fields, namespace mistakes, unknown runtime
references, missing files, and ambiguous versions must fail during install.

There are two contracts:

1. **Source package format** — what Hub, Git, Local, Built-in, or any future
   installer source provides.
2. **Installed node format** — what Zebflow materializes under `repo/nodes/`
   and indexes at runtime.

All installer sources normalize into the installed node format. No source is
allowed to bypass validation.

### Source package format

Source packages always use `definition.json`, even for one node. A single-node
package is a package whose `nodes` array has length `1`.

```
{package-slug}/
├── definition.json              # package manifest: one or more nodes
├── icon.svg                     # optional package icon
├── icons/
│   └── {node}.svg               # optional per-node icons
├── functions/
│   └── *.zf.json                # composite implementation functions
├── wasm/
│   └── *.wasm                   # WASM implementation modules
└── README.md                    # optional human docs
```

`definition.json` uses a single package format:

```json
{
  "format": "zebflow-package-v2",
  "package": "sim",
  "version": "0.1.0",
  "title": "Simulation",
  "description": "Simulation nodes.",
  "icon": "icon.svg",
  "credentials": [],
  "functions": {},
  "wasm": {},
  "nodes": []
}
```

Version nomenclature:

- `format` is the package manifest format id. Use letters and numbers exactly:
  `"zebflow-package-v2"`.
- `version` is the package release version. Use SemVer numbers:
  `"0.1.0"`, `"1.0.0"`, `"1.2.3"`.
- Node kind versions are not encoded in `kind`. A kind stays stable; package
  `version` changes.

### Installed node format

The installed format is Zebflow-owned. Users and Hub packages should not rely on
authoring it directly.

```
repo/nodes/{node-slug}/
├── node.json                    # canonical single-node manifest
├── icon.svg                     # resolved node icon, if provided
├── pipeline.zf.json             # composite only
└── module.wasm                  # WASM only
```

Multi-node source packages are exploded into one installed node folder per
visible node. Runtime indexing reads only validated installed nodes.

---

## Native Nodes

Rust modules at `src/pipeline/nodes/basic/**`.

Module shape:

```rust
pub const NODE_KIND: &str = "n.category.action";
pub fn definition() -> NodeDefinition { /* ... */ }
pub struct Config { /* deserialized from node config JSON */ }
pub struct Node { config: Config }
impl NodeHandler for Node { /* execute_async */ }
```

Registration:
- Add module under `src/pipeline/nodes/basic/`
- Export in `mod.rs`
- Add `definition()` to `builtin_node_definitions()`
- Add dispatch arm in execution engine

Native nodes cover: triggers, HTTP, database clients, filesystem, web response, auth, logic, AI, realtime, mapserver, script execution.

---

## Composite Nodes

A composite node package is a collection of related nodes built from function pipelines. One package can contain multiple nodes sharing credentials, lifecycle hooks, and reusable functions. The package uses the same field vocabulary as native Rust nodes.

### Source package shape

```
{package-slug}/
├── definition.json
├── icon.svg
├── icons/*.svg
└── functions/*.zf.json
```

### definition.json

```json
{
  "format": "zebflow-package-v2",
  "package": "telegram",
  "version": "1.0.0",
  "title": "Telegram",
  "description": "Telegram Bot — send messages, receive updates.",
  "icon": "icon.svg",

  "credentials": [
    {
      "kind": "telegram_bot",
      "title": "Telegram Bot",
      "fields": [
        { "key": "token", "type": "password", "required": true, "label": "Bot Token" }
      ],
      "placeholders": { "BOT_TOKEN": "token" },
      "config_key": "bot_credential_id"
    }
  ],

  "functions": {
    "send-message":     "functions/send-message.zf.json",
    "register-webhook": "functions/register-webhook.zf.json",
    "delete-webhook":   "functions/delete-webhook.zf.json",
    "transform-update": "functions/transform-update.zf.json"
  },

  "nodes": [
    {
      "kind": "n.c.trigger.tg",
      "source": "composite",
      "title": "Telegram Trigger",
      "description": "Receive Telegram bot updates.",
      "icon": "icons/trigger.svg",
      "trigger": {
        "type": "webhook",
        "on_message": "transform-update"
      },
      "lifecycle": {
        "on_activate": "register-webhook",
        "on_deactivate": "delete-webhook"
      },
      "definition": {
        "input_pins": [],
        "output_pins": ["out"],
        "config_schema": {},
        "fields": [],
        "layout": [],
        "dsl_flags": []
      }
    },
    {
      "kind": "n.c.tg.send",
      "source": "composite",
      "title": "Telegram Send Message",
      "icon": "icons/send.svg",
      "main": "send-message",
      "definition": {
        "input_pins": ["in"],
        "output_pins": ["out", "error"],
        "config_schema": {},
        "fields": [],
        "layout": [],
        "dsl_flags": []
      }
    }
  ]
}
```

### Package = node collection

A single package installs multiple related nodes. Installing "Telegram" gives the user `n.c.trigger.tg`, `n.c.tg.send`, `n.c.tg.send.photo`, etc. All nodes share the same credential pool. Uninstalling the package removes all nodes.

### Functions = reusable pipelines

The `functions` map declares named function pipelines. Each is a standard pipeline graph file starting with `n.trigger.function`. Any node in the package can reference any function by name:

- **Action nodes**: `"main": "send-message"` — runs on normal execution.
- **Trigger nodes**: `"on_message": "transform-update"` — transforms inbound events.
- **Lifecycle**: `"on_activate": "register-webhook"` — runs when pipeline activates.

Functions are shared. The same `register-webhook` function can be used by `on_activate` and by a self-managed renewal schedule (via `n.trigger.cron` calling it periodically).

### Credentials: define and require

Two credential modes in `credentials[]`:

**Define** — package creates a new credential kind with fields and placeholders:

```json
{
  "kind": "telegram_bot",
  "title": "Telegram Bot",
  "fields": [
    { "key": "token", "type": "password", "required": true, "label": "Bot Token" }
  ],
  "placeholders": { "BOT_TOKEN": "token" },
  "config_key": "bot_credential_id"
}
```

**Require** — package needs an existing credential kind (postgres, openai, etc.):

```json
{
  "kind": "postgres",
  "config_key": "db_credential_id"
}
```

Rule: if `fields` is present, it defines a new kind. If absent, it requires an existing kind.

Inner pipelines reference credentials through `$config`:
- Defined credentials: use `$placeholder.BOT_TOKEN` in expressions (HTTP URLs, script).
- Required credentials: pass `{{ $config.db_credential_id }}` to inner nodes that accept `credential_id`.

### Credential placeholder model

Nodes never see secret values. The composite manifest maps credential fields to named placeholders. The platform resolves placeholders only at the final consumer (HTTP client, script sandbox), never in payload, trace, or logs.

**Flow:**

1. User configures composite node, selects credential.
2. Composite runtime reads the credential, builds placeholder map from manifest: `$placeholder.BOT_TOKEN` → actual value.
3. Inner pipeline nodes receive only placeholder references.
4. Platform HTTP client resolves placeholders right before network send — only place real values exist.
5. Traces and logs show: `url=https://api.telegram.org/bot$placeholder.BOT_TOKEN/sendMessage` — safe by default.

**Security guarantees:**

- No `$secret` variable exists. Nodes cannot access raw credential values.
- Placeholder references are safe to log, trace, and include in error messages.
- Only the platform HTTP client and script sandbox resolve placeholders, at the last possible moment.
- The composite manifest explicitly declares which credential fields are accessible as placeholders — no blanket access.

### Lifecycle wiring

Nodes that need external setup/teardown declare lifecycle hooks. Each hook references a function by name.

```json
{
  "kind": "n.c.trigger.tg",
  "lifecycle": {
    "on_activate": "register-webhook",
    "on_deactivate": "delete-webhook"
  }
}
```

| Hook | When it runs | Purpose |
|------|-------------|---------|
| `on_activate` | Pipeline activated | Register webhook, subscribe, open connection |
| `on_deactivate` | Pipeline deactivated | Unregister webhook, unsubscribe, cleanup |

**On activate** receives: `$placeholder` (credentials), `$config` (node settings), `$platform.public_url` (server's public URL for constructing callback URLs).

**Renewal scheduling** — there is no separate renewal system. The `on_activate` function handles initial registration. If the service requires periodic renewal, the package includes a `n.trigger.cron` pipeline that calls the same registration function on a schedule. This is self-managed using existing pipeline primitives. The `n.kv` store holds state (webhook IDs, timestamps) between runs. `on_deactivate` cleans up everything including removing the scheduled pipeline.

**On restart** — `on_activate` re-runs for all active pipelines. Registration functions should be idempotent (most external APIs support this — e.g., Telegram `setWebhook` replaces the previous webhook).

### Trigger nodes

A composite trigger node declares how it receives events:

```json
{
  "kind": "n.c.trigger.tg",
  "trigger": {
    "type": "webhook",
    "on_message": "transform-update"
  }
}
```

| Trigger type | How events arrive | Examples |
|---|---|---|
| `webhook` | External service POSTs to Zebflow webhook URL | Telegram, GitHub, Stripe |
| `ws` | Messages in a Zebflow WS room | Internal real-time events |
| `ws_client` | Connected to external WebSocket server | Discord Gateway, crypto feeds |
| `cron` | Scheduled execution | Polling services (Notion) |

The `on_message` function transforms raw inbound payloads into clean typed output before the pipeline continues.

### Sandbox

Composite inner pipelines run in a restricted sandbox. The sandbox is enforced by the platform — inner pipelines can only use allowed node kinds. No permission prompt needed because dangerous nodes are structurally unavailable.

**Allowed nodes:**

| Category | Nodes |
|---|---|
| Triggers | `n.trigger.webhook`, `n.trigger.ws`, `n.trigger.ws.client`, `n.trigger.cron`, `n.trigger.function`, `n.trigger.kv.subscribe` |
| HTTP | `n.http.request` |
| WebSocket | `n.ws.client.send`, `n.ws.emit` |
| Script | `n.script` |
| KV | `n.kv.set`, `n.kv.get`, `n.kv.del`, `n.kv.exists`, `n.kv.incr`, `n.kv.expire`, `n.kv.publish` |
| Logic | `n.match`, `n.transform`, `n.merge`, `n.function.call`, `n.delay` |

**Blocked nodes** (belong in apps or project pipelines):

| Category | Why blocked |
|---|---|
| Database (`n.pg.*`, `n.mysql.*`, `n.sqlite.*`) | Data access — use an app instead |
| Filesystem (`n.fs.*`, `n.table.*`) | Storage access — use an app instead |
| Auth (`n.auth.*`) | Token minting — project pipeline concern |
| Platform internals | Never available to composites |

**Design principle:** if a composite needs database or filesystem access, it's not a node — it's an app. Composite nodes talk to external APIs and transform data. Apps are a separate hub type that can access everything.

The platform validates all function pipeline files on install. If any inner pipeline uses a blocked node kind, installation is rejected.

### KV scoping for composites

Composite inner pipelines use `n.kv` for state (webhook IDs, cursors, timestamps). To prevent collision and snooping between composites, KV keys are auto-scoped per node instance.

When a composite's inner pipeline does `n.kv.set --key "webhook_id"`, the platform writes:

```
{owner}/{project}/__node/{pipeline_path}/{node_id}/webhook_id
```

The composite never sees or controls the full key. Two instances of the same Telegram trigger in different pipelines get different scopes. A composite cannot construct a key outside its own scope.

Regular `n.kv` usage in native pipelines remains project-scoped (current behavior).

### Expression scope

Inside composite inner pipelines:

| Variable | Contents |
|----------|----------|
| `$input` | Outer node's input payload (from upstream node) |
| `$config` | Outer node's config values (credential_id, etc.) |
| `$placeholder` | Opaque credential references (resolved only by platform consumers) |
| `$trigger` | Trigger snapshot from the outer pipeline |
| `$nodes` | Completed node outputs from the outer pipeline |
| `$platform.public_url` | Server's public URL (for constructing webhook callbacks) |

### Execution model

**Action nodes (`main`):**

1. Outer node receives payload and config.
2. Composite runtime reads credential from config, builds placeholder map.
3. Engine loads the function pipeline referenced by `main`.
4. Outer payload, config, and placeholder map injected into execution context.
5. Inner graph runs with isolated trace context.
6. Terminal output becomes outer node output. Errors route to `error` pin.

**Trigger nodes (`on_message`):**

1. External event arrives (webhook POST, WS message, etc.).
2. Platform matches event to composite trigger node.
3. Engine loads the `on_message` function pipeline.
4. Raw event payload injected as `$input`.
5. Function output becomes the pipeline's trigger payload.

**Lifecycle hooks (`on_activate` / `on_deactivate`):**

1. Pipeline activation/deactivation event fires.
2. Platform finds composite nodes with lifecycle hooks.
3. Engine loads the lifecycle function pipeline.
4. `$config`, `$placeholder`, `$platform.public_url` injected.
5. Function runs (e.g., calls `setWebhook` API).
6. Output stored in node-scoped KV if needed.

### Key rules

- `definition` uses the same field type names as native nodes: `Text`, `Textarea`, `Number`, `Checkbox`, `Select`, `Datalist`, `CodeEditor`, `MethodButtons`, `CopyUrl`, `MultiCheckbox`, `KeyValuePairs`, `ParamsBuilder`.
- `data_source` uses `credentials:{kind}` to filter by any credential kind.
- `credentials` array declares credential types and placeholder mappings. Installing the package auto-registers new credential types.
- All function pipelines must start with `n.trigger.function`.
- Node kinds must be `n.c.*` and globally unique. Cannot override native nodes.
- Icons must be local SVG within the package.
- Inner pipelines may only use sandbox-allowed node kinds.
- Nested composites are supported. Circular references rejected at install time.
- Error propagation: inner pipeline failure routes to `error` output pin. If no `error` pin declared, the composite node itself fails with `PipelineError`.
- Versioning: packages carry a `version` field. Update replaces the catalog entry. No hot-swap mid-execution.

### Legacy installed-node manifest

The existing runtime can still read project-local `repo/nodes/{node-slug}/node.json`
because that is the installed node manifest. New source packages should not use
`node.json`; all source packages should use `definition.json` with a `nodes`
array.

### Open design items

1. **`$placeholder` as expression variable** — ✅ **DONE**. Resolved at runtime by `build_composite_placeholder_map()` in `src/pipeline/engines/basic.rs`. Script sandbox receives placeholders through `ctx.placeholder`.

2. **Installation mechanism** — API endpoint for uploading packages (zip/tar). Security scan validates sandbox compliance (no blocked nodes). MCP tool `node_install` follows. UI: hub page with upload + browse.

3. **Storage location** — project-level composites stored under `repo/nodes/{package-slug}/`. Official composites embedded in the binary via `PLATFORM_COMPOSITE_NODE_ASSETS`. Both use identical package format.

4. **`$platform.public_url`** — requires `ZEBFLOW_PUBLIC_URL` env var or platform config setting. Available to lifecycle hooks for constructing webhook callback URLs.

5. **Icon and asset serving** — `GET /api/projects/{owner}/{project}/nodes/{node-slug}/icon.svg` serves per-node or package icons. Official composites served from binary static assets.

---

## Foundational Gaps — Third-Party Integration Infrastructure

Status of infrastructure needed to cover 35+ major third-party services.

### Gap 1: Composite Trigger Node

**Status**: ✅ Designed (v2 package spec above)

Composite trigger nodes are defined in the v2 package format with `trigger.type` (webhook/ws/ws_client/cron) and `on_message` function for payload transformation. The package spec covers all inbound integration patterns.

### Gap 2: `on_activate` / `on_deactivate` Hooks

**Status**: ✅ Designed (v2 lifecycle wiring above)

Lifecycle hooks are function references in the package manifest. Platform runs them during pipeline activate/deactivate. Expression scope includes `$placeholder`, `$config`, `$platform.public_url`.

### Gap 3: `$platform.public_url`

**Status**: Not implemented

Requires `ZEBFLOW_PUBLIC_URL` env var or platform config. Available to lifecycle hooks for constructing webhook callback URLs:

```
Webhook URL = $platform.public_url + /wh/{owner}/{project}/ + webhook_path
```

### Gap 4: Challenge-Response Handler

**Status**: Not implemented

Challenge patterns:

| Pattern | Services |
|---------|----------|
| GET with `hub.challenge` echo | Meta/Facebook, Instagram, WhatsApp |
| POST with `{"challenge":"..."}` JSON echo | Slack, Monday.com |
| POST with challenge + HMAC encrypt response | Zoom, Twitter/X CRC |
| GET with `SubscribeURL` to confirm | AWS SNS |
| POST with `X-Hook-Secret` echo in header | Asana |
| POST with `validationToken` query param echo | Microsoft Teams Graph |
| HEAD request (must return 200) | Trello, Mandrill, LINE |

Can be handled by the `on_message` function in composite trigger nodes — the function inspects the payload and returns the appropriate challenge response. No separate platform infrastructure needed.

### Gap 5: Inbound Signature Verification

**Status**: Partially done (webhook auth: JWT, HMAC, API key. WS auth: same interface.)

Remaining verification methods:

| Method | Services |
|--------|----------|
| HMAC-SHA256 of (timestamp + body) | Stripe, Calendly, Zoom (replay protection) |
| HMAC-SHA1 | Trello, Twilio, Mandrill (legacy) |
| RSA / Certificate | PayPal, AWS SNS |
| ECDSA P-256 | SendGrid |
| Ed25519 | Discord interactions |

These can be handled by composite trigger `on_message` functions using `n.script` for verification, or by extending the native webhook auth with additional methods.

### Gap 6: Webhook Renewal

**Status**: ✅ Solved by design

No separate renewal scheduler. Composite packages self-manage renewal using existing primitives:
- `on_activate` function registers the webhook, stores state in node-scoped `n.kv`
- A `n.trigger.cron` pipeline in the package calls the same registration function periodically
- `on_deactivate` cleans up everything

The registration function is shared — `on_activate` and the cron schedule both call it.

### Gap 7: Polling Trigger

**Status**: ✅ Solved by design

Composite trigger with `"type": "cron"`. The `on_message` function polls the external API, diffs with last known state (stored in node-scoped `n.kv`), and emits changes.

### Gap 8: Pipeline-Level State

**Status**: ✅ Solved by `n.kv` with auto-scoping

No separate `$store` concept. Composite nodes use `n.kv` with automatic per-node-instance scoping. Keys are namespaced to `{owner}/{project}/__node/{pipeline}/{node_id}/{key}`. Composites cannot access other nodes' state.

### Gap 9: Persistent Connection Manager

**Status**: ✅ Done

`n.trigger.ws.client` + `n.ws.client.send` provide outbound WebSocket connections with auto-reconnect, heartbeat, and exponential backoff. `WsClientManager` in `src/infra/ws_client/mod.rs`.

### Implementation Status

| Gap | Status | What remains |
|-----|--------|-------------|
| #1 Composite trigger | Designed | Runtime implementation of v2 package loading |
| #2 Lifecycle hooks | Designed | Runtime implementation in activate/deactivate path |
| #3 Public URL | Not done | `ZEBFLOW_PUBLIC_URL` env var + `$platform.public_url` |
| #4 Challenge-response | Designed | Handled by `on_message` functions — no platform infra needed |
| #5 Signature verification | Partial | Extend webhook auth or handle in `n.script` |
| #6 Renewal | Solved | Self-managed via cron + shared functions |
| #7 Polling | Solved | Cron trigger + KV cursor |
| #8 Pipeline state | Solved | `n.kv` with auto-scoping |
| #9 Persistent connections | Done | `n.trigger.ws.client` + `n.ws.client.send` |

### Third-Party Registration Pattern Reference

Every external service falls into one of these patterns:

| Type | Description | Count | Examples |
|------|-------------|-------|---------|
| **A** | Simple webhook registration — POST a URL, get events | ~15 | Telegram, GitHub, Stripe, Shopify, Airtable, GitLab, PayPal |
| **B** | Verification challenge — service sends a challenge you must echo back | ~10 | Slack, Meta/FB, Zoom, AWS SNS, Asana, Monday.com, MS Teams |
| **C** | OAuth + subscription — OAuth flow first, then subscribe | ~12 | Slack, Google, HubSpot, MS Teams, Zoom, LinkedIn (layers on A/B) |
| **D** | Polling only — no webhook support | 1 | Notion |
| **E** | Persistent connection — WebSocket/SSE/gRPC | 3 | Discord Gateway, Salesforce CometD, Twitter/X Stream |
| **F** | Pub/Sub — subscribe to a topic/queue | 4 | AWS SNS, GCP Pub/Sub, Azure Event Grid, Gmail |

Services with mandatory renewal: MS Teams (1-72h), Google Drive/Calendar (~24h), Gmail (7d), Airtable (7d), Jira (30d), Twitter CRC (24h).

Services with thin events (push-then-poll): Google Drive/Calendar, Gmail, Airtable, Asana, Stripe (optional).

Unique patterns: Twilio (request-response — webhook must return TwiML), Salesforce (no HTTP webhooks — CometD/gRPC only), Zendesk (webhook-as-action, not subscription), Shopify (multi-transport: HTTP/EventBridge/Pub/Sub).

### OAuth2 Credential Flow

**Status**: ✅ Done

Full OAuth2 implementation exists in `src/platform/services/credential.rs`:
- Code exchange: `exchange_oauth2_code()`
- Auto-refresh: `get_valid_oauth2_token()` — checks expiry, refreshes automatically
- HTTP request node: detects `kind: "oauth2"`, auto-injects Bearer token with refresh
- Credential type: `client_id`, `client_secret`, `token_url`, `access_token`, `refresh_token`

---

## Webhook Header Forwarding Registry

The webhook trigger (`n.trigger.webhook`) exposes inbound request headers to pipelines via `$trigger.headers`. The platform maintains a default forwarding whitelist of third-party signature, event type, delivery ID, and timestamp headers. Projects can extend this whitelist in project settings.

This registry covers 120+ services across all major categories.

### Default Forwarded Header Prefixes

Any header matching these prefixes is forwarded automatically:

| Prefix | Services |
|--------|----------|
| `X-GitHub-*` | GitHub (Event, Delivery, Hook-ID, Hook-Installation-Target-ID, Hook-Installation-Target-Type) |
| `X-Hub-Signature*` | GitHub, Meta/FB, WhatsApp, Instagram, Jira, Bitbucket, Snyk, Intercom, Mastodon, YouTube WebSub, Labelbox, Chargebee Retention |
| `X-Gitlab-*` | GitLab (Event, Token, Event-UUID, Instance, Webhook-UUID) |
| `X-Gitea-*` | Gitea (Delivery, Event, Event-Type, Signature) |
| `X-Gogs-*` | Gogs (Delivery, Event, Signature) |
| `X-Shopify-*` | Shopify (Topic, Hmac-Sha256, Shop-Domain, Webhook-Id, Event-Id, Triggered-At, API-Version, Test) |
| `X-WC-Webhook-*` | WooCommerce (Source, Topic, Resource, Event, Signature, ID, Delivery-ID) |
| `X-Slack-*` | Slack (Signature, Request-Timestamp) |
| `X-Twilio-*` | Twilio SMS/Voice (Signature) + SendGrid (Email-Event-Webhook-Signature, Email-Event-Webhook-Timestamp) |
| `X-Razorpay-*` | Razorpay (Signature, Event-Id) |
| `X-Todoist-*` | Todoist (Hmac-SHA256, Delivery-ID) |
| `X-HubSpot-*` | HubSpot (Signature, Signature-Version, Signature-v3, Request-Timestamp) |
| `X-Contentful-*` | Contentful (Topic, Webhook-Name, Crn, Idempotency-Key, Bulk-Action-Id, Bulk-Action-Type) |
| `X-CIO-*` | Customer.io (Signature, Idempotency-Key, Delivery-ID) |
| `X-CC-*` | Coinbase Commerce (Webhook-Signature) |
| `Twitch-Eventsub-*` | Twitch (Message-Id, Message-Type, Message-Signature, Message-Timestamp, Message-Retry, Subscription-Type, Subscription-Version) |
| `X-Buildkite-*` | Buildkite (Token, Event, Signature, Request) |
| `X-Telegram-*` | Telegram (Bot-Api-Secret-Token) |
| `X-Viber-*` | Viber (Content-Signature, Auth-Token) |
| `PAYPAL-*` | PayPal (Auth-Algo, Cert-URL, Transmission-ID, Transmission-Sig, Transmission-Time) |
| `Sentry-Hook-*` | Sentry (Resource, Timestamp, Signature) |
| `Linear-*` | Linear (Delivery, Event, Signature) |
| `Klaviyo-*` | Klaviyo (Webhook-Id, Signature, Timestamp) |
| `x-amz-sns-*` | AWS SNS (message-type, message-id, topic-arn, subscription-arn) |
| `aeg-*` | Azure Event Grid (event-type, subscription-name, delivery-count, data-version, metadata-version, output-event-id) |
| `X-Zoho-*` | Zoho (Webhook-Signature) |
| `x-zm-*` | Zoom (signature, request-timestamp, trackingid) |
| `X-Pipedrive-*` | Pipedrive (Signature) |
| `X-Cal-*` | Cal.com (Signature-256, webhook-version) |
| `X-Drone-*` | Drone CI (Event) |
| `x-docusign-*` | DocuSign (signature, signature-1) |
| `x-pagerduty-*` | PagerDuty (signature) |
| `X-Apify-*` | Apify (Webhook, Webhook-Dispatch-Id, Request-Origin) |
| `X-Labelbox-*` | Labelbox (Id, Event) |
| `X-Snyk-*` | Snyk (Event) |
| `X-MLflow-*` | MLflow (Signature, Timestamp) |
| `X-Fivetran-*` | Fivetran (Signature-256) |
| `Revolut-*` | Revolut (Request-Timestamp, Signature) |
| `X-Afterpay-*` | Afterpay/Clearpay (Request-Signature) |
| `X-CS-*` | CrowdStrike (Primary-Signature) |
| `X-Databricks-*` | Databricks (Signature) |
| `X-FS-*` | FastSpring (Signature) |
| `X-Bolt-*` | Bolt checkout (HMAC-SHA256) |
| `X-Affirm-*` | Affirm (Signature) |

### Default Forwarded Exact Headers

| Header | Service |
|--------|---------|
| `Stripe-Signature` | Stripe |
| `Paddle-Signature` | Paddle |
| `Calendly-Webhook-Signature` | Calendly |
| `Typeform-Signature` | Typeform |
| `circleci-signature` | CircleCI |
| `circleci-event-type` | CircleCI |
| `figma-signature` | Figma |
| `webhook-id` | Standard Webhooks spec (Svix, OpenAI, Replicate, Google Vertex AI, GitLab 19+) |
| `webhook-timestamp` | Standard Webhooks spec |
| `webhook-signature` | Standard Webhooks spec |
| `x-vercel-signature` | Vercel |
| `X-Webhook-Signature` | Netlify, 2Checkout |
| `X-Mandrill-Signature` | Mandrill/Mailchimp |
| `X-Postmark-Signature` | Postmark |
| `X-Notion-Signature` | Notion |
| `X-LI-Signature` | LinkedIn |
| `X-Trello-Webhook` | Trello |
| `X-Hook-Secret` | Asana, Zoho |
| `X-Hook-Signature` | Asana |
| `X-Signature` | ClickUp, Lemon Squeezy |
| `X-Signature-Ed25519` | Discord |
| `X-Signature-Timestamp` | Discord |
| `X-Event-Name` | Lemon Squeezy |
| `X-Event-Key` | Bitbucket |
| `X-Hook-UUID` | Bitbucket |
| `X-Request-UUID` | Bitbucket |
| `X-Attempt-Number` | Bitbucket |
| `x-square-hmacsha256-signature` | Square |
| `square-environment` | Square |
| `x-monday-signature` | Monday.com |
| `X-Sonar-Webhook-HMAC-SHA256` | SonarQube |
| `X-SonarQube-Project` | SonarQube |
| `X-JFrog-Event-Auth` | Artifactory |
| `X-Ghost-Signature` | Ghost |
| `X-Grafana-Alerting-Signature` | Grafana |
| `x-webflow-signature` | Webflow |
| `x-webflow-timestamp` | Webflow |
| `x-line-signature` | LINE |
| `X-Dropbox-Signature` | Dropbox |
| `x-pandadoc-signature` | PandaDoc |
| `x-airtable-signature` | Airtable |
| `X-Supabase-Event-Signature` | Supabase |
| `Plaid-Verification` | Plaid |
| `sm-signature` | SurveyMonkey |
| `Sm-Apikey` | SurveyMonkey |
| `x-tiktok-signature` | TikTok |
| `x-tiktok-timestamp` | TikTok |
| `x-webhook-id` | PagerDuty |
| `x-webhook-subscription` | PagerDuty |
| `HmacSignature` | Adyen |
| `Protocol` | Adyen |
| `X-Atlassian-Webhook-Identifier` | Jira Cloud |
| `X-Body-Signature` | Intercom |
| `X-Webhook-Secret` | Hugging Face |
| `Webhook-Signature` | GoCardless |
| `Gitguardian-Signature` | GitGuardian |
| `Timestamp` | GitGuardian |
| `x-webhook-signature` | Socket.dev |
| `x-mcd-signature` | Monte Carlo |
| `x-atlan-signing-secret` | Atlan |
| `x-esrihook-signature` | Esri/ArcGIS |
| `X-Signature-SHA256` | Wise (TransferWise) |
| `recurly-signature` | Recurly |
| `Affirm-Signature` | Affirm |
| `scale-callback-auth` | Scale AI |
| `Original-Status` | Crawlbase |
| `PC-Status` | Crawlbase |
| `rid` | Crawlbase |

### Always Forwarded (standard)

These standard headers are always forwarded regardless of whitelist:

- `content-type`
- `accept`
- `user-agent`
- `x-forwarded-for`
- `x-real-ip`
- `referer`
- `origin`

### Never Forwarded

- `cookie` — handled by platform auth layer
- `authorization` — handled by platform auth layer (JWT/Bearer verification)

### Project Settings Extension

Projects can add custom header names or prefixes to the forwarding whitelist via project settings. This allows support for niche or internal services not covered by the default list.

---

## WASM Nodes

Sandboxed binary nodes via Extism. For capabilities that need custom code outside native + composite reach.

### Source package shape

```
{package-slug}/
├── definition.json
├── icon.svg
├── icons/*.svg
└── wasm/*.wasm
```

One WASM module can serve many nodes. The package declares modules once, then
each node points at a module and export.

### definition.json

```json
{
  "format": "zebflow-package-v2",
  "package": "sim",
  "version": "0.1.0",
  "title": "Simulation",
  "description": "Simulation nodes.",
  "icon": "icon.svg",
  "credentials": [],
  "functions": {},
  "wasm": {
    "core": {
      "module": "wasm/sim_core.wasm",
      "runtime": "zebflow-wasm-json-v1",
      "exports": {
        "des": "sim_des_run",
        "queue": "sim_queue_run"
      }
    }
  },
  "nodes": [
    {
      "kind": "n.wasm.sim.des",
      "source": "wasm",
      "wasm": "core",
      "export": "des",
      "title": "DES Simulation",
      "description": "Runs a deterministic discrete-event simulation kernel.",
      "icon": "icons/des.svg",
      "definition": {
        "input_pins": ["in"],
        "output_pins": ["out", "error"],
        "config_schema": {},
        "input_schema": {},
        "output_schema": {},
        "fields": [],
        "layout": [],
        "dsl_flags": [],
        "ui_category": "simulation",
        "ui_category_label": "Simulation"
      }
    }
  ]
}
```

### Key rules

- Kind must be `n.wasm.*` and globally unique.
- The referenced `wasm.{name}.module` file must exist.
- The referenced node `wasm` and `export` must exist.
- The same `NodeDefinition` fields drive catalog indexing, DSL flags, forms, docs, and LLM context.
- WASM starts with zero host capabilities. Any future host function must be declared by package policy and validated before install.
- WASM modules access credentials through the same placeholder model as composites — `$placeholder.NAME` resolved by the host, never raw secret values.

### WASM data movement strategy

Native, composite, and WASM nodes must expose the same visible node I/O
contract. The node slug (`n.*`, `n.c.*`, `n.wasm.*`) identifies the
implementation, not a different user-facing data model. The engine owns
normalization under the hood.

| Data size / node kind | Native `n.*` | Composite `n.c.*` | WASM `n.wasm.*` |
|---|---|---|---|
| Small JSON / control data | Inline JSON is fine. Native nodes run in the engine process and can handle ordinary payload values directly. | Inline JSON is fine. A composite boundary should not change the visible payload shape. | Inline JSON is fine through `zebflow-wasm-json-v1`. One host/guest copy is acceptable for small control data. |
| Large or file-like data | Prefer FileRef, ZebFS paths, table/geo storage, or node output files. Native nodes should stream/read from storage when the data is file-like. | Prefer FileRef/ZebFS paths through the composite boundary. Inner graphs should not expand file-like data into huge JSON unless the workflow explicitly asks for rows/previews. | Prefer FileRef metadata plus WASM host read/write strategy. Do not use giant JSON as the long-term bulk-data path. |

Current supported lanes:

- Inline JSON for small WASM inputs and outputs.
- Generic node output files via `__zf_files`; the engine materializes those
  entries into ZebFS and downstream nodes receive FileRef JSON.

Near-term WASM homework:

- Define host read calls so WASM can read FileRef/PayloadHandle data by chunks
  or byte ranges, similar to S3 range/object-store reads.
- Define host write calls so WASM can write large generated output directly to
  ZebFS/FileRef without returning the whole body through the JSON ABI.
- Keep emit/stream as TODO until the read/write path is stable.

Warning: `__zf_files` is a useful bridge, not the final large-WASM-data
solution. It prevents downstream payload amplification, but the file body still
returns through the JSON ABI once. Truly large WASM output needs host file-write
calls.

### Script-level contract

Script usage is not a separate package format. If a node wants to expose script
helpers, it must declare that in the same `NodeDefinition` contract:

- `script_available`
- `script_bridge`
- `ai_tool`
- `dsl_flags`
- `fields`

Native, composite, and WASM nodes all appear in one node contract document. If a
capability is not declared there, scripts, agents, DSL tooling, and UI must treat
it as unavailable.

---

## Custom Credential Types

### Built-in credential kinds

| Kind | Fields | Used by |
|------|--------|---------|
| `postgres` | host, port, database, user, password, sslmode | `pg.query` |
| `mysql` | host, port, database, user, password | `mysql.query` |
| `openai` | api_key, base_url, model | `n.ai.prompt`, `n.ai.agent` |
| `http` | base_url, token | `n.http.request` |
| `github` | username, token, git_name, git_email | git operations |
| `gitlab` | url, username, token, git_name, git_email | git operations |
| `jwt_signing_key` | algorithm, secret, private_key, auth_roles | `n.auth.token.create`, webhook auth |
| `oauth2` | client_id, client_secret, token_url, access_token, refresh_token | `n.http.request` |
| `secure_request` | variables[] | `n.http.request` |
| `browser_browserless` | url, token | `n.browser.run` |
| `hmac` | secret | webhook auth |
| `api_key` | key | webhook auth |
| `custom` | json (freeform) | any node via manual config |

### Custom credential type definition

Composite and WASM packages declare custom credential types in `definition.json → credentials[]`:

```json
{
  "kind": "telegram_bot",
  "title": "Telegram Bot",
  "description": "Bot token from @BotFather.",
  "fields": [
    { "key": "bot_token", "label": "Bot Token", "type": "password", "required": true, "help": "Get from @BotFather." }
  ],
  "placeholders": {
    "BOT_TOKEN": "bot_token"
  }
}
```

- `fields` — defines the credential creation form (what the user fills in).
- `placeholders` — maps named placeholders to credential secret fields. Inner pipeline nodes access secrets only through these placeholders. Only declared fields are exposed.

Field types for credential form:

| Type | Renders as |
|------|-----------|
| `text` | Plain text input |
| `password` | Masked input |
| `select` | Dropdown (requires `options: [{value, label}]`) |
| `textarea` | Multi-line text |
| `number` | Numeric input |
| `tags` | Tag list input |

Optional field properties: `placeholder`, `help`, `default`, `generate` (`"random_hex_32"` for auto-generated secrets), `options` (for select type).

### Credential type registry

1. Built-in types ship as hardcoded definitions (same format, compiled in).
2. Custom types come from installed composite/WASM packages.
3. API: `GET /api/projects/{owner}/{project}/credential-types` returns merged list.
4. Credentials UI renders any type dynamically from its definition.

### Node data source binding

Nodes reference credentials by kind filter:

- Native Rust: `data_source: Some(NodeFieldDataSource::Credentials("telegram_bot".into()))`
- Composite/WASM JSON: `"data_source": "credentials:telegram_bot"`

The `credentials:{kind}` pattern replaces per-kind enum variants. Existing variants (`CredentialsPostgres`, `CredentialsOpenAi`, etc.) remain as aliases for backward compatibility.

### Secret access model

Nodes never access raw credential values. The placeholder model provides structural security — secrets cannot leak because they never enter the node-visible domain.

**Three boundaries:**

1. **Composite manifest** — declares exactly which credential fields are accessible and under which placeholder names. No blanket access to the full secret object.
2. **Inner pipeline nodes** — use `$placeholder.NAME` references only. The expression engine passes these through as literal strings, never resolves them to real values.
3. **Platform consumers** — HTTP client and script sandbox resolve placeholders at the final moment (network send, runtime eval). Resolved values are never written to trace, log, payload, or error output.

**Why placeholders, not direct secret access:**

| Concern | Placeholder `$placeholder.X` | Direct `$secret.x` |
|---------|------------------------------|---------------------|
| Trace/log leakage | Impossible — value never in node context | Requires redaction scanner on all output |
| Error message leakage | Shows placeholder name, safe | Real value in `"HTTP 404 at ...bot123:ABC/..."` |
| Payload leakage | Can't leak what nodes don't have | Script can `return { token: $secret.x }` downstream |
| Access control | Manifest declares exactly which fields | Blanket access to all secret fields |
| Infrastructure | No redaction system needed | Must build and trust system-wide scanner |

Placeholder model is both leaner and safer. Security is structural, not reactive.

**Script sandbox enforcement:**

If a script inside a composite attempts to exfiltrate a resolved placeholder value (e.g., returning it in the output payload), the execution signal system can detect and terminate the execution. Placeholder values resolved inside the script sandbox are marked — any attempt to include them in the output payload raises a signal error. This is a future enforcement layer on top of the structural model.

**HTTP error re-masking:**

The platform HTTP client must catch errors from resolved requests and re-mask any resolved placeholder values back to their placeholder names before returning errors to the pipeline. Example: `"HTTP 404 at https://api.telegram.org/bot123:ABC/..."` becomes `"HTTP 404 at https://api.telegram.org/bot$placeholder.BOT_TOKEN/..."`.

**Native node credential access:**

Built-in native nodes (`pg.query`, `n.http.request`, etc.) access credentials internally through the credential service — compiled Rust code with controlled access paths. The placeholder model applies only to composite and WASM nodes where the inner pipeline is user-authored.

### Credential type availability

All credential types use the same JSON definition format — there is no "native" vs "composite" distinction for credential types. Unlike nodes (where native nodes have compiled Rust runtime), credential types are pure data definitions: a list of fields with types, labels, and help text. `postgres` and `telegram_bot` are the same thing structurally.

Credential type definitions are served from the server via API. The credentials UI renders any type dynamically from its definition — no hardcoded frontend schemas.

Custom credential types are introduced through composite/WASM package installation. When a package declares credential types in its `credentials[]` manifest, those types are registered in the project credential system. Standalone custom credential type creation (without a package) is a future addition.

---

## Field Renderer Contract

Fields are the Project Studio UX contract. Same vocabulary across native, composite, and WASM definitions.

### Field type registry

- `Text` — short string, supports `{{ expr }}` inline
- `Textarea` — long string, supports `{{ expr }}` inline. Not a fallback for structured config.
- `CodeEditor` — code/SQL/JSON with `language`, `span`, `default_value`, `sidebar`
- `Number` — numeric: counts, sizes, timeouts, limits
- `Checkbox` — boolean
- `Select` — closed single-choice, optionally with `data_source`
- `Datalist` — editable text with live suggestions
- `MethodButtons` — compact choice buttons (HTTP methods)
- `CopyUrl` — read-only derived URL
- `Section` — grouping label
- `MultiCheckbox` — closed multi-choice
- `KeyValuePairs` — repeated key/value object
- `ClaimsPairs` — JWT claim map with exposure control
- `ParamsBuilder` — function parameter schema builder
- `SecureRequestBindings` — credential variable binding editor
- `MatchCases` — case/default route editor

### Layout

- `Field("name")` — single field
- `Row(["a", "b"])` — horizontal group
- `Col(["a", "b"])` — vertical group

### Rules

- Do not create redundant renderers. Reuse existing types.
- Do not use `Textarea` to avoid designing correct UI.
- New renderers require design discussion: name the missing interaction, saved shape, affected nodes, why existing fields are insufficient.
- A node is not ready when users must know hidden JSON or DSL conventions to configure it.

---

## Namespace Rules

| Source | Namespace | Override rule |
|--------|-----------|---------------|
| Native | `n.*` | Cannot be overridden |
| Composite | `n.c.*` | Cannot collide with native. Nested domains allowed: `n.c.aws.s3.upload` |
| WASM | `n.wasm.*` | Cannot collide with native. Nested domains allowed: `n.wasm.pdf.watermark` |

## Catalog Merge

One merged catalog for project-studio, DSL, MCP, and validator:

1. Native definitions from `builtin_node_definitions()`
2. Installed composite definitions
3. Installed WASM definitions

Source metadata preserved so UI can badge native vs composite vs WASM.

---

## Distribution Taxonomy

### Node Tiers

| Tier | Label | Namespace | Storage | Uninstall | Developer |
|------|-------|-----------|---------|-----------|-----------|
| OF | Official Foundation | `n.*` | Binary (compiled Rust) | No | Platform developer |
| OC | Official Composite | `n.c.*` | Binary (embedded assets) | No | Platform developer |
| OW | Official WASM | `n.wasm.*` | Installable package | Yes | Platform developer |
| CC | Community Composite | `n.c.*` | `repo/nodes/` | Yes | Community |
| CW | Community WASM | `n.wasm.*` | `repo/nodes/` | Yes | Community |

The API response includes `source` (`"native"`, `"composite"`, `"wasm"`) and `tier` (`"official"`, `"community"`) for every node. The UI can badge nodes by tier.

### Credential types

| Category | What it is | Storage |
|----------|------------|---------|
| Official | Zebflow-shipped types (postgres, openai, jwt, etc.) | Server-defined (same JSON format as custom) |
| Public/custom | Types from composite/WASM packages or user-created | Package manifest or project-level definition |

All credential types use the same JSON definition format regardless of origin. There is no compiled/native distinction for credential types — they are pure data definitions.

### Key rules

- Official foundational nodes (`n.*`) cannot be overridden or uninstalled by any source.
- Official composite nodes (OC) are embedded in the binary using `PLATFORM_COMPOSITE_NODE_ASSETS`. They cannot be overridden or uninstalled. They use the same package format as community composites.
- Community composites and WASM nodes are installed per-project and can be uninstalled.
- Official credential types use the same definition format as custom types. The server serves all types through one merged API. The UI renders any type dynamically.
- LLMs can author composite nodes because the format is the same everywhere — no special knowledge needed for "official" vs "community" packaging.

---

## Developing Official Foundation Nodes (OF)

Platform developers creating native Rust nodes.

### Steps

1. Create a Rust module at `src/pipeline/nodes/basic/{category}/{node_name}.rs`.
2. Define `NODE_KIND`, implement `definition() -> NodeDefinition`, and implement `NodeHandler`.
3. Register in `src/pipeline/nodes/basic/mod.rs`:
   - Add `pub mod {node_name};` to the module tree.
   - Add `{node_name}::definition()` to `builtin_node_definitions()`.
4. Add dispatch arm in `src/pipeline/engines/basic.rs`.
5. Add an icon SVG at `src/platform/web/assets/node-icons/zebflow/{kind}.svg`.
6. Add `include_bytes!()` entry to `PLATFORM_NODE_ICON_ASSETS` in `src/platform/web/embedded.rs`.
7. `cargo check` — verify clean compile.

### Module shape

```rust
pub const NODE_KIND: &str = "n.category.action";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Action Title".to_string(),
        description: "What this node does.".to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string()],
        // ... fields, layout, schemas, dsl_flags ...
        ..Default::default()
    }
}

pub struct Config { /* deserialized from node config JSON */ }
pub struct Node { config: Config }
impl NodeHandler for Node { /* execute_async */ }
```

---

## Developing Official Composite Nodes (OC)

Platform developers creating reusable pipeline-based nodes that ship with the binary.

### Steps

1. Create a package directory at `composites/{slug}/` (e.g. `composites/telegram/`).
2. Add package files:
   - `definition.json` — source package manifest.
   - `functions/*.zf.json` — inner function pipelines.
   - `icon.svg` and `icons/*.svg` — package and node catalog icons.
3. Add `include_bytes!()` entries to `PLATFORM_COMPOSITE_NODE_ASSETS` in `src/platform/web/embedded.rs`:

```rust
EmbeddedAsset { path: "telegram/definition.json", bytes: include_bytes!("../../../composites/telegram/definition.json") },
EmbeddedAsset { path: "telegram/icon.svg", bytes: include_bytes!("../../../composites/telegram/icon.svg") },
EmbeddedAsset { path: "telegram/functions/send-message.zf.json", bytes: include_bytes!("../../../composites/telegram/functions/send-message.zf.json") },
```

4. `cargo check` — the embedded composite is loaded at startup by `NodeRegistryService::load_embedded_composites()`.

### Key properties

- Same source package format as community composites — `definition.json` plus declared artifacts.
- Cannot be uninstalled (protected by `NodeRegistryService::is_official()`).
- Merged into the node catalog alongside native builtins — appears in Project Studio node picker.
- Inner pipeline uses `$placeholder` for credential secrets — same security model as community composites.

---

## Developing Community Nodes (CC / CW)

Community developers creating composite or WASM nodes installed per-project.

### Steps

1. Create the package:
   - **Composite (CC)**: `definition.json` + `functions/*.zf.json` + icons.
   - **WASM (CW)**: `definition.json` + `wasm/*.wasm` + icons.
   - Mixed packages may contain both composite and WASM nodes.
2. Install via API:

```bash
curl -X POST -H "Content-Type: application/json" \
  -b /tmp/zf.txt \
  -d @install_payload.json \
  http://localhost:10610/api/projects/{owner}/{project}/nodes/install
```

Payload shape:
```json
{
  "package": { /* parsed definition.json and package artifacts */ }
}
```

3. Verify installation:

```bash
curl -b /tmp/zf.txt \
  http://localhost:10610/api/projects/{owner}/{project}/nodes/by-kind/n.c.my.node
```

4. Uninstall:

```bash
curl -X DELETE -b /tmp/zf.txt \
  http://localhost:10610/api/projects/{owner}/{project}/nodes/uninstall/n.c.my.node
```

### Key rules

- Kind must be `n.c.*` (composite) or `n.wasm.*` (WASM).
- Cannot override native or official composite nodes.
- Installed in `repo/nodes/{slug}/` — project-scoped.
- Can be uninstalled at any time.
- API response includes `tier: "community"` to distinguish from official nodes.
