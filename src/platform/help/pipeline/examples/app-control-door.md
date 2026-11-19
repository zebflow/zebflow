# Let an app build inside its own project (control door)

## What this builds

A door through which an app built on Zebflow adds to itself while it runs: an
admin of the app (or the app's own agent) sends commands, and Zebflow writes
the files and pipelines they ask for. It writes them only inside a fence: one
folder, one route prefix, the operations the fence opens, and the credentials
it delegates. The app's main code, its other credentials and the project's
shared stores stay out of reach.

```
app admin ─▶ POST /zf/control   { tool, arguments }
              trigger.webhook --auth jwt --role admin
              zebflow.command.run --credential zf_modules
                 ├─ inside the fence ─▶ done, answered
                 └─ past it ─────────▶ refused, 403 with the reason
```

## 1. The fence (Studio → Credentials, kind `zebflow_control`)

```yaml
scope: modules/
routes: /m/
tools: [file_read, file_write, pipeline_register, pipeline_activate, pipeline_deactivate]
delegate: [pg_modules]
nodes_mode: allow
nodes: [trigger.webhook, trigger.schedule, postgres.query.run, http.response.fetch, javascript.script.run, web.response.send, logic.*]
record_as: app-builder
```

Only a maintainer makes or changes it, so nothing sent through the door can
widen it. `pg_modules` is a Postgres credential whose database user may write
only in the schemas meant for modules: the database's own permissions fence
the data.

## 2. The door

```
[t] trigger.webhook --route /zf/control --method POST --auth jwt --credential app_session --role admin
[c] zebflow.command.run --credential zf_modules --command "{{ input.webhook.body }}" --as "{{ input.webhook.auth.sub }}"
[ok] web.response.send --body "{{ input.command }}"
[no] web.response.send --status 403 --body "{{ input.command }}"
[t] -> [c]
[c] -> [ok]
[c]:error -> [no]
```

## 3. What the app sends

One command, or a list run in order, in the shape the MCP tools take:

```json
[
  { "tool": "pipeline_register", "arguments": {
      "file_rel_path": "modules/rain/api/latest.zf.json",
      "body": "| trigger.webhook --route /m/rain/latest --method GET | postgres.query.run --credential pg_modules -- \"SELECT * FROM mod_rain.hourly ORDER BY at DESC LIMIT 24\" | web.response.send --body \"{{ input.query.rows }}\"" } },
  { "tool": "pipeline_activate", "arguments": { "file_rel_path": "modules/rain/api/latest.zf.json" } }
]
```

The answer is `{ ok: true, results: [...] }`, one result per command. A refused
command answers 403 with its index, tool and reason, for example
`command 0 (pipeline_register): node 't' answers at '/admin/x', outside /m/`.
The commands before it stay done.

## What a made pipeline may not do

It starts with a webhook, schedule or manual trigger; a webhook answers only
under the route prefix; it uses only delegated credentials, written out by id;
and it never uses `zebflow.*`. Pipeline files change only through the pipeline
operations, never `file_write`. Which other nodes it may use is the fence's
`nodes` list: `allow` lets through only those listed, triggers included (as
above); `deny`, the default,
lets through everything but those listed, and an empty list denies the
project's shared stores (`sekejap`, `sqlite`, `kv`, `fs`, `table`, `function`).
This is checked when it is registered and again when it is switched on.

Every change shows in the change record as `control · app-builder` with the
person from `--as` (`GET /changes`, MCP `change_list`).
