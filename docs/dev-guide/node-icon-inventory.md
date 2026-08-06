# Zebflow Node Icon Inventory

Draft status: planning and production checklist.

This document owns the first pass of Zebflow node identity: what icons are
needed, where each icon should come from, and which nodes each icon supports.

## Goals

- Every node icon is a 128x128 SVG.
- Zebflow-native nodes use one coherent visual family: iconic, high contrast,
  strong silhouettes, not generic app logos.
- Third-party/service nodes prefer official SVG assets when brand licensing
  allows it.
- PNG-only assets are converted to SVG with a repeatable trace step.
- Missing/ambiguous service icons get custom generated source artwork, sliced if
  needed, then traced into SVG.
- Every icon can carry a tiny 2-5 character bottom-right key when that improves
  scanning, for example `PG`, `S3`, `DDB`, `DRV`, `API`.

## Visual System

Zebflow-native icons should share:

- square 128 viewBox
- no outer card baked into the SVG unless the mark needs a background
- bold monoline or filled geometric symbol, optimized for 24-40 px display
- bottom-right key in pixel-style lettering only when needed
- category accent color compatible with current editor colors:
  - triggers: green
  - logic: cyan
  - web/network: orange or rose
  - data/memory: amber or teal
  - files/media: blue or violet
  - security: amber or purple
  - AI: indigo

The native family should look like one product set even when individual symbols
are different. Reuse base motifs:

- trigger: ingress arrow or event spark
- logic: branch/merge glyph
- data: table/cylinder/cell
- memory/pubsub: chip/cell/broadcast
- web: globe/socket/response panel
- file/media: document/image/archive
- security: key/shield/hash
- AI: spark/chip/waveform

## Acquisition Methods

| Method | Meaning | When to use |
|---|---|---|
| official-svg | Download vendor/official SVG, normalize to 128x128. | Brand/service nodes with usable SVG assets. |
| official-png-trace | Download official PNG, trace to SVG. | Vendor only publishes PNG or raster toolkit. |
| devicon | Use Devicon SVG/font source, convert/embed as 128 SVG. | Developer/database logos such as Postgres, MySQL, Redis, MongoDB, GitHub if acceptable. |
| simple-icons | Use Simple Icons SVG with brand color, normalize. | SaaS/developer brands with simple-icons coverage. |
| n8n-source | Use n8n node icon file as a source reference only after license review. | Matching n8n import compatibility; do not blindly vendor. |
| generated-trace | Generate raster sheet, slice, trace to SVG. | Zebflow-native nodes and service icons with no suitable official mark. |
| direct-svg | Draw SVG directly. | Simple native symbols where deterministic geometry is better than tracing. |

## Proposed Files

```text
src/platform/web/assets/node-icons/
  manifest.json
  zebflow/
    n.trigger.webhook.svg
    n.logic.if.svg
    ...
  brands/
    google-drive.svg
    apify.svg
    ...
tools/node-icons/
  fetch_icons.py
  trace_icon.py
  normalize_svg.py
  build_manifest.py
```

The web editor should use `manifest.json` to map node kind or integration key to
an asset path, with `nodeColor(kind)` remaining as a fallback only.

## Pipeline

1. `fetch_icons.py` reads an icon source manifest and downloads official SVG/PNG
   assets into `tmp/node-icons/source/`.
2. `normalize_svg.py` rewrites SVGs to a 128x128 viewBox, removes fixed width and
   height, and keeps only the mark. It must not recolor official multicolor marks.
3. `trace_icon.py` converts PNG to SVG. Prefer Inkscape CLI when available:
   `inkscape input.png --export-type=svg --export-filename output.svg`. For flat
   logos, fall back to a potrace/autotrace-style Python path if installed.
4. `build_manifest.py` validates every final SVG:
   - file exists
   - root is `<svg>`
   - viewBox is `0 0 128 128`
   - no raster `<image>` unless explicitly allowed
   - manifest entry has license/source/method/status

## Current Zebflow Nodes

Status values:

- todo: icon not produced yet
- source-ready: acquisition path is known
- designed: visual direction defined
- done: final SVG exists and is validated

| Icon key | Supports node kind(s) | Title | Method | Key | Status | Direction |
|---|---|---:|---|---|---|---|
| zeb-trigger-webhook | `n.trigger.webhook` | Webhook Trigger | direct-svg | WH | todo | Inbound arrow into route bracket/globe. |
| zeb-trigger-schedule | `n.trigger.schedule` | Schedule Trigger | direct-svg | CRON | todo | Clock face plus tick spark. |
| zeb-trigger-manual | `n.trigger.manual` | Manual Trigger | direct-svg | RUN | todo | Play button/hand-trigger hybrid. |
| zeb-trigger-ws | `n.trigger.ws` | WebSocket Trigger | direct-svg | WS | todo | Socket pulse entering node. |
| zeb-trigger-memsubscribe | `n.trigger.memsubscribe` | Mem Subscribe | direct-svg | SUB | todo | Memory cell with broadcast receiver. |
| zeb-trigger-weberror | `n.trigger.weberror` | Web Error Trigger | direct-svg | ERR | todo | Broken route with alert notch. |
| zeb-trigger-function | `n.trigger.function` | Function Trigger | direct-svg | FN | todo | Function bracket as ingress. |
| zeb-script | `n.script` | Script | direct-svg | JS | todo | Code chevrons inside node diamond. |
| zeb-function-call | `n.function.call` | Call Function | direct-svg | CALL | todo | Function bracket with outbound arrow. |
| zeb-logic-if | `n.logic.if` | If | direct-svg | IF | todo | Two-way branch, true/false pins. |
| zeb-logic-match | `n.logic.match` | Match | direct-svg | M | todo | Switch/fork with multiple pins. |
| zeb-logic-collect | `n.logic.collect` | Collect | direct-svg | JOIN | todo | Many arrows merging into tray. |
| zeb-logic-foreach | `n.logic.foreach` | Foreach | direct-svg | EACH | todo | Loop ring around item cells. |
| zeb-logic-reduce | `n.logic.reduce` | Reduce | direct-svg | RED | todo | Funnel/accumulator symbol. |
| zeb-logic-retry | `n.logic.retry` | Retry | direct-svg | TRY | todo | Circular arrow with failure pin. |
| zeb-http-request | `n.http.request` | HTTP Request | direct-svg | HTTP | todo | Outbound request arrow/globe. |
| zeb-browser-run | `n.browser.run` | Browser Run | direct-svg | BRS | todo | Browser viewport with play/cursor. |
| zeb-web-render | `n.web.render` | Web Render UI entry | direct-svg | SSR | todo | Template panel rendering into page. |
| zeb-web-response | `n.web.response` | Web Response | direct-svg | RESP | todo | Response panel with return arrow. |
| zeb-web-static-generate | `n.web.static.generate` | Web Static Generate | direct-svg | SSG | todo | Page stack with static spark. |
| zeb-web-docs-generate | `n.web.docs.generate` | Web Docs Generate | direct-svg | DOCS | todo | Docs tree to generated page stack. |
| zeb-ws-sync-state | `n.ws.sync_state` | WS Sync State | direct-svg | SYNC | todo | Bidirectional socket/state cells. |
| zeb-ws-emit | `n.ws.emit` | WS Emit | direct-svg | EMIT | todo | Broadcast pulse leaving node. |
| zeb-sekejap-query | `n.sekejap.query` | Sekejap Query | direct-svg | SJQ | todo | Zebflow table/query mark. |
| zeb-sekejap-mutate | `n.sekejap.mutate` | Sekejap Mutate UI entry | direct-svg | SJM | todo | Zebflow table with write pen/bolt. |
| zeb-pg-query | `n.pg.query` | Postgres Query | official-svg | PG | done | Real PostgreSQL elephant mark, normalized to 128x128. |
| zeb-sqlite-query | `n.sqlite.query` | SQLite Query | brand-svg | SQL | done | Real SQLite square mark, normalized to 128x128. |
| zeb-sqlite-mutate | `n.sqlite.mutate` | SQLite Mutate | brand-svg+badge | SQL+ | done | Real SQLite square mark with small mutate badge. |
| zeb-mem-set | `n.mem.set` | Mem Set | direct-svg | SET | todo | Memory cell with down/write arrow. |
| zeb-mem-get | `n.mem.get` | Mem Get | direct-svg | GET | todo | Memory cell with up/read arrow. |
| zeb-mem-exists | `n.mem.exists` | Mem Exists | direct-svg | HAS | todo | Memory cell with check bit. |
| zeb-mem-del | `n.mem.del` | Mem Del | direct-svg | DEL | todo | Memory cell with cut/delete notch. |
| zeb-mem-expire | `n.mem.expire` | Mem Expire | direct-svg | TTL | todo | Memory cell with clock. |
| zeb-mem-incr | `n.mem.incr` | Mem Incr | direct-svg | +1 | todo | Counter cell with plus/step. |
| zeb-mem-publish | `n.mem.publish` | Mem Publish | direct-svg | PUB | todo | Memory cell broadcasting. |
| zeb-fs-save | `n.fs.save` | FS Save | direct-svg | SAVE | todo | Object with downward write slot. |
| zeb-fs-compress | `n.fs.compress` | FS Compress | direct-svg | ZIP | todo | Archive clamp around objects. |
| zeb-fs-decompress | `n.fs.decompress` | FS Decompress | direct-svg | UNZ | todo | Archive opening into objects. |
| zeb-fs-pdf-convert | `n.fs.pdf.convert` | PDF Convert | direct-svg | PDF | todo | PDF page split into object assets. |
| zeb-fs-thumbnail | `n.fs.thumbnail` | FS Thumbnail | direct-svg | IMG | todo | Image frame shrinking to small tile. |
| zeb-ai-agent | `n.ai.agent` | AI Agent | direct-svg | AI | todo | Spark/chip with tool pin. |
| zeb-ai-tts | `n.ai.tts` | AI TTS | direct-svg | TTS | todo | Waveform/speaker with AI spark. |
| zeb-auth-token-create | `n.auth.token.create` | Create Auth Token | direct-svg | JWT | todo | Key/shield signing token. |
| zeb-crypto | `n.crypto` | Crypto | direct-svg | HASH | todo | Hash/key glyph with lock. |

## n8n-Inspired Backlog

This backlog is based on the current n8n integrations page sorted by popularity
and n8n docs for core nodes. It is not a claim that Zebflow implements these
nodes today; it is an import/compatibility icon backlog.

| Priority | Icon key | n8n/Zebflow target | How it works at node level | Method | Key | Status |
|---:|---|---|---|---|---|---|
| 1 | brand-google-sheets | Google Sheets | Spreadsheet rows read/update/append; often paired with HTTP/OpenAI. | official-svg or simple-icons | SHET | todo |
| 2 | zeb-ai-agent | AI Agent | LLM agent with tools and iterative execution. Reuse native `n.ai.agent`. | direct-svg | AI | todo |
| 3 | zeb-http-request | HTTP Request | Generic REST call with method, URL, auth, headers, query/body, timeout, pagination. Reuse `n.http.request`. | direct-svg | HTTP | todo |
| 4 | brand-gmail | Gmail | Consume/send/search Gmail API resources; trigger variant polls/fetches mail. | official-svg or simple-icons | GML | todo |
| 5 | brand-openai | OpenAI | Model calls, embeddings, chat, speech/image operations depending on node. | official-svg or simple-icons | OAI | todo |
| 6 | brand-slack | Slack | Send/read messages and channel/user operations. | official-svg or simple-icons | SLK | todo |
| 7 | brand-telegram | Telegram / Telegram Trigger | Bot messaging plus trigger updates. | official-svg or simple-icons | TGM | todo |
| 8 | brand-google-gemini | Google Gemini | Gemini model calls/tools. | official-svg or simple-icons | GEM | todo |
| 9 | brand-anthropic | Anthropic | Claude model calls/tooling. | official-svg or simple-icons | ANT | todo |
| 10 | brand-airtable | Airtable / Airtable Trigger | Table records read/create/update/delete; trigger watches records. | official-svg or simple-icons | AIR | todo |
| 11 | brand-google-drive | Google Drive / Drive Trigger | File/folder operations and file change triggers. | official-svg or simple-icons | DRV | todo |
| 12 | brand-webhook | Webhook | Receive external HTTP request and start workflow. Reuse `n.trigger.webhook`. | direct-svg | WH | todo |
| 13 | brand-excel | Microsoft Excel 365 | Workbook/sheet rows read/update. | official-svg or simple-icons | XLS | todo |
| 14 | brand-notion | Notion / Notion Trigger | Page/database operations and change triggers. | official-svg or simple-icons | NTN | todo |
| 15 | brand-supabase | Supabase | Database/storage/auth API operations. | official-svg or simple-icons | SUP | todo |
| 16 | brand-discord | Discord | Messages, channels, guild interaction. | official-svg or simple-icons | DSC | todo |
| 17 | brand-postgres | Postgres | SQL query action. Reuse/extend `n.pg.query`. | devicon or official-svg | PG | source-ready |
| 18 | brand-email | Send Email | SMTP/email send action. | direct-svg | MAIL | todo |
| 19 | brand-mysql | MySQL | SQL query/mutate integration. | devicon | MYSQL | source-ready |
| 20 | brand-spreadsheet-file | Spreadsheet File | Parse/write CSV/XLSX-like binary files. | direct-svg | CSV | todo |
| 21 | brand-github | GitHub | Repos/issues/PRs/actions integration. | official-svg or devicon | GH | source-ready |
| 22 | brand-google-calendar | Google Calendar / Calendar Trigger | Event read/create/update and schedule triggers. | official-svg or simple-icons | CAL | todo |
| 23 | brand-mongodb | MongoDB | Document database operations. | devicon or official-svg | MDB | source-ready |
| 24 | brand-mssql | Microsoft SQL Server | SQL query/mutate integration. | devicon or official-svg | MSSQL | source-ready |
| 25 | brand-openweathermap | OpenWeatherMap | Weather lookup API. | simple-icons or generated-trace | WX | todo |
| 26 | brand-graphql | GraphQL | GraphQL request execution. | simple-icons or direct-svg | GQL | todo |
| 27 | brand-hubspot | HubSpot | CRM/contact/deal operations. | official-svg or simple-icons | HUB | todo |
| 28 | brand-x | X/Twitter | Social posting/search/user operations. | official-svg or simple-icons | X | todo |
| 29 | brand-redis | Redis | Key-value/cache/pubsub operations. | devicon | RDS | source-ready |
| 30 | brand-aws-s3 | AWS S3 | Object storage put/get/list/delete. | official-svg | S3 | todo |
| 31 | brand-aws-lambda | AWS Lambda | Invoke/manage serverless functions. | official-svg | LAMB | todo |
| 32 | brand-aws-dynamodb | AWS DynamoDB | NoSQL table get/put/query/update. | official-svg | DDB | todo |
| 33 | brand-aws-sqs | AWS SQS | Queue send/receive/delete. | official-svg | SQS | todo |
| 34 | brand-aws-sns | AWS SNS | Topic publish/subscription notifications. | official-svg | SNS | todo |
| 35 | brand-apify | Apify | Run actors, read datasets/key-value stores, respond to Apify events. | official-svg or simple-icons | APFY | todo |

## Official Source Notes

- n8n currently says it has 1690 integrations and exposes a popularity-sorted
  list led by Google Sheets, AI Agent, HTTP Request, Gmail, OpenAI, Slack,
  Telegram, Google Gemini, Anthropic, Airtable, Google Drive, Webhook, Excel,
  Notion, Supabase, Discord, Postgres, Send Email, MySQL, GitHub, and more.
- n8n defines nodes as workflow building blocks; trigger nodes start workflows
  and action nodes perform operations during a workflow.
- n8n node metadata includes an `icon` parameter and recommends SVG icons, with
  PNG accepted at 60x60 when SVG is unavailable.
- AWS publishes architecture icon packages and states releases happen Q1, Q2,
  and Q3, with no Q4 release.
- Devicon provides developer/database/tool icons in SVG/font variants and is
  already referenced by the Zebflow app through `devicons.css`.
- Simple Icons provides thousands of brand SVGs and can be used as a convenient
  brand source after trademark/license review.

## Immediate Checklist

- [ ] Add `src/platform/web/assets/node-icons/manifest.json`.
- [ ] Add first Zebflow-native prototype batch: webhook, schedule, script, if,
      http request, pg query, web response, mem set, file save, ai agent,
      auth token, crypto.
- [ ] Decide whether native icons are hand-drawn direct SVG or generated sheet
      plus trace. Recommendation: direct SVG for the first batch so the theme is
      controllable.
- [ ] Add asset validation script for 128x128 SVGs.
- [ ] Update pipeline editor types to include `icon?: string` on
      `NodeCatalogEntry`.
- [ ] Update node rendering to use manifest icons and keep current colors as
      fallback.
- [ ] Add brand download manifest for priority n8n-compatible services.
- [ ] Run license/trademark review before bundling brand marks into Zebflow.

## Sources

- n8n integrations page: https://n8n.io/integrations/
- n8n integrations docs: https://docs.n8n.io/integrations/
- n8n HTTP Request docs: https://docs.n8n.io/integrations/builtin/core-nodes/n8n-nodes-base.httprequest/
- n8n Webhook docs: https://docs.n8n.io/integrations/builtin/core-nodes/n8n-nodes-base.webhook/
- n8n node icon parameter docs: https://docs.n8n.io/integrations/creating-nodes/build/reference/node-base-files/standard-parameters/
- AWS architecture icons: https://aws.amazon.com/architecture/icons/
- Devicon: https://github.com/devicons/devicon
- Simple Icons: https://www.npmjs.com/package/simple-icons
