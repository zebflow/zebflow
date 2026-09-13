# Agentic Scheduling (AI + Cron)

## What this builds

Scheduled pipelines that invoke `n.ai.agent` (aliased `zebtune`) to analyze
data, generate summaries, classify items, or make decisions. Results are
stored in Sekejap and optionally sent out over HTTP.

---

## Pipelines

1. `CRON every hour` → fetch recent events → AI summary → store
2. `CRON daily` → aggregate summaries → AI report → send to a webhook
3. `CRON every 15 min` → fetch unclassified queue items → AI classify → update rows
4. `GET /admin/reports` → list AI-generated summaries → render page

---

## Tables

```sql
CREATE TABLE events (_key TEXT PRIMARY KEY, ts INTEGER, kind TEXT, detail JSON)
CREATE TABLE ai_summaries (_key TEXT PRIMARY KEY, summary TEXT, patterns JSON, anomalies JSON, period TEXT, generated_at INTEGER)
CREATE TABLE incoming_queue (_key TEXT PRIMARY KEY, payload JSON, urgency TEXT, category TEXT, processed BOOLEAN)
```

`events` is assumed to be populated by whatever emits them (another pipeline,
an import job); this doc only reads it.

---

## `n.ai.agent` (zebtune) — how it actually takes input

There is no `--prompt` and no `--input_path`. The agent's goal/query is
extracted from the payload's `message`, `body`, `text`, or `query` field — so
a `script` node builds that field before the agent runs. Instructions that
should hold for every call (format, tone, constraints) go in `--system-prompt`,
not in the message.

| Flag | Description |
|------|-------------|
| `--system-prompt "…"` | Standing instructions for the agent, e.g. "reply with strict JSON only". |
| `--credential <id>` | OpenAI-compatible credential id. |
| `--model <name>` | Override the model set on the credential. |
| `--output-mode final_only` | Skip the iteration/tool trace, keep only the answer. |
| `--max-iterations <n>` | Cap on LLM iterations (direct mode, default 5). |

Direct-mode output is `{ response: "…" }` — a string. If the prompt asked for
JSON, `JSON.parse(input.response)` in the next `script` node.

---

## DSL

### hourly-data-summary — fetch + AI summarize

```
| trigger.schedule --cron "0 * * * *"
| script -- "return { cutoff: Date.now() - 3600000 }"
| sekejap.query --params "{{ [input.cutoff] }}" -- "SELECT * FROM events WHERE ts > $1"
| script -- "return { message: 'Summarize these events, count patterns, and flag anything unusual. Events: ' + JSON.stringify(input.rows) }"
| n.ai.agent --credential my-llm --system-prompt "You are an operations analyst. Reply with strict JSON only: { summary: string, patterns: array, anomalies: array }." --output-mode final_only
| script -- "const r = JSON.parse(input.response); return { key: 'summary-' + Date.now(), summary: r.summary, patterns: r.patterns || [], anomalies: r.anomalies || [], period: 'hourly', generated_at: Date.now() }"
| sekejap.query --read-only false --params "{{ [input.key, input.summary, input.patterns, input.anomalies, input.period, input.generated_at] }}" -- "INSERT INTO ai_summaries (_key, summary, patterns, anomalies, period, generated_at) VALUES ($1, $2, $3, $4, $5, $6)"
```

### daily-metrics-report — aggregate + report + send

```
| trigger.schedule --cron "0 8 * * *"
| script -- "return { cutoff: Date.now() - 86400000 }"
| sekejap.query --params "{{ [input.cutoff] }}" -- "SELECT * FROM ai_summaries WHERE generated_at > $1"
| script -- "return { message: 'Write a daily operations report in markdown from these hourly summaries — executive summary, key metrics, trends, recommendations. Summaries: ' + JSON.stringify(input.rows) }"
| n.ai.agent --credential my-llm --system-prompt "Reply with the markdown report only, no commentary." --output-mode final_only
| kv.get --key report_webhook_url --out-key webhook_url --durable
| http.request --url "{{ input.webhook_url }}" --method POST --body "{{ { report: input.response } }}"
```

`kv.get` merges `{ webhook_url }` into the payload alongside `input.response`
— nothing is lost. Set `report_webhook_url` once via `kv.set` (or a Settings
page) before this pipeline runs; there is no `env` scope to read a URL from.

### queue-classifier — AI classify and route

```
| trigger.schedule --cron "*/15 * * * *"
| sekejap.query -- "SELECT * FROM incoming_queue WHERE processed = false LIMIT 10"
| script -- "return { message: 'Classify each item by urgency (high, medium, low) and category. Reply with a strict JSON array of { key, urgency, category }, where key is the _key of that item. Items: ' + JSON.stringify(input.rows) }"
| n.ai.agent --credential my-llm --system-prompt "Reply with strict JSON only, no prose." --output-mode final_only
| script -- "return { classified: JSON.parse(input.response) }"
| logic.foreach --items-expr "input.classified"
| sekejap.query --read-only false --params "{{ [$item.urgency, $item.category, $item.key] }}" -- "UPDATE incoming_queue SET urgency = $1, category = $2, processed = true WHERE _key = $3"
```

`logic.foreach` fans out one run per array element; `$item` stays in scope
for every downstream node in that run even after `sekejap.query` replaces
`input`.

### admin-reports — view AI reports

```
| trigger.webhook --path /admin/reports --method GET
| sekejap.query -- "SELECT * FROM ai_summaries ORDER BY generated_at DESC LIMIT 30"
| script -- "return { reports: input.rows }"
| web.response --template pages/admin-reports.tsx
```

---

## Nodes Used

- `trigger.schedule` — cron-based scheduling (`0 * * * *` = hourly, `0 8 * * *` = daily 8am)
- `sekejap.query` — SQL against Sekejap; output is `{ columns, rows, row_count, affected_rows, duration_ms }`
- `script` — build the AI `message`, parse its JSON reply, shape rows for insert
- `n.ai.agent` (`zebtune`) — analysis, classification, report generation
- `logic.foreach` — one downstream run per classified item
- `kv.get` / `kv.set` — hold the webhook URL (there is no `env` scope)
- `http.request` — send the report to an external webhook
- `web.response` — admin reporting page

---

## Templates Needed

- `pages/admin-reports.tsx` — display AI-generated summaries with timestamps

> A script cannot set the response. It returns a value; the graph decides what
> happens next. Branch with `logic.if` and let `web.response` answer —
> `--status`, `--location`, `--set-cookie`. See
> `help("pipeline/examples/webhook-restapi-postgres")` § Answering with a status.
