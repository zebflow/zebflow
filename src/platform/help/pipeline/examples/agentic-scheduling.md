# Agentic Scheduling (AI + Cron)

## What this builds

Scheduled pipelines that invoke an AI agent (zebtune) to analyze data, generate summaries, take actions, or make decisions. Results stored in Sekejap and optionally sent via HTTP webhook/email.

---

## Pipelines

1. `CRON every hour` → fetch latest data → AI analysis → store summary
2. `CRON daily` → aggregate metrics → AI report → send to webhook
3. `CRON every 15 min` → check queue → AI classify → route to handler
4. `GET /admin/reports` → list AI-generated reports → render page

---

## DSL

### hourly-data-summary — fetch + AI summarize

```
| trigger.schedule --cron "0 * * * *"
| sekejap.query --table events --op scan
| script -- "const recent = input.filter(e => e.ts > Date.now() - 3600000); return { count: recent.length, events: recent, period: 'last_hour' }"
| zebtune --prompt "Summarize these events. Count patterns. Flag anything unusual. Return JSON with: summary (string), patterns (array), anomalies (array)." --input_path /
| script -- "return { ...input, generated_at: Date.now(), period: 'hourly' }"
| sekejap.query --table ai_summaries --op upsert
```

### daily-metrics-report — aggregate + report + send

```
| trigger.schedule --cron "0 8 * * *"
| sekejap.query --table ai_summaries --op scan
| script -- "const yesterday = input.filter(s => s.generated_at > Date.now() - 86400000); return { summaries: yesterday, date: new Date().toISOString().slice(0,10) }"
| zebtune --prompt "Generate a daily operations report from these hourly summaries. Include: executive summary, key metrics, trends, recommendations. Format as markdown." --input_path /summaries
| http.request --url "{{env.REPORT_WEBHOOK_URL}}" --method POST --body_path /
```

### queue-classifier — AI classify and route

```
| trigger.schedule --cron "*/15 * * * *"
| sekejap.query --table incoming_queue --op scan
| script -- "const unprocessed = input.filter(i => !i.processed); return { items: unprocessed.slice(0, 10) }"
| zebtune --prompt "Classify each item by urgency (high/medium/low) and category. Return array of {id, urgency, category} objects." --input_path /items
| script -- "return { classified: input, ts: Date.now() }"
| sekejap.query --table classified_queue --op upsert
```

### admin-reports — view AI reports

```
| trigger.webhook --path /admin/reports --method GET
| sekejap.query --table ai_summaries --op scan
| script -- "return { reports: input.sort((a,b)=>b.generated_at-a.generated_at).slice(0,30) }"
| web.render --template-path pages/admin-reports.tsx --route /admin/reports
```

---

## Nodes Used

- `trigger.schedule` — cron-based scheduling (`0 * * * *` = hourly, `0 8 * * *` = daily 8am)
- `sekejap.query` — read queues, write summaries (scan, upsert)
- `script` — filter, transform, time window calculations
- `zebtune` — AI agent for analysis, classification, generation
- `http.request` — send reports/alerts to external webhooks
- `web.render` — admin reporting page

---

## zebtune Node Flags

| Flag | Description |
|------|-------------|
| `--prompt "text"` | Instruction for the AI agent |
| `--input_path /` | JSON pointer into payload to pass as AI input |
| `--model claude-haiku-4-5-20251001` | Optionally specify model (defaults to configured) |

The AI output becomes the next node's `input`. Structure your prompts to return parseable JSON when the output needs further processing.

---

## Templates Needed

- `pages/admin-reports.tsx` — display AI-generated summaries with timestamps
