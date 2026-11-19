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
tools: [file_read, file_write, pipeline_register, pipeline_activate, pipeline_deactivate, credential_list, credential_create]
delegate: [pg_modules]
nodes_mode: allow
nodes: [trigger.webhook, trigger.schedule, postgres.query.run, http.response.fetch, javascript.script.run, web.response.send, logic.*]
manage_kinds: [postgres]
manage_limit: 50
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
`command 0 (pipeline_register): node 1 ('t') answers at '/admin/x', outside /m/`.
The commands before it stay done. A `file_write` answers the path and size,
not the content.

Routes are paths, not hosts: a made webhook answers on the hosts whose routes
reach its prefix (`help("web")`, Addressing), not on every host of the project.

## 4. Credentials the door manages

With `credential_create` opened and `manage_kinds` set, the app adds its own
connections, for example ten outside PostgreSQL databases:

```json
{ "tool": "credential_create", "arguments": {
    "credential_id": "ext-census", "kind": "postgres", "title": "Census",
    "secret": { "host": "db.example.com", "port": "5432", "database": "census",
                "user": "reader", "password": "…", "sslmode": "require" } } }
```

- `credential_list` answers the delegated credentials and the ones this door
  made: id, kind, title, the fields that are not passwords. Never a secret.
- `credential_update` (`secret` is the fields to change) and
  `credential_delete` touch only the ones this door made.
- Only kinds in `manage_kinds`, never `zebflow_control`, at most
  `manage_limit`. Every address must be outside the machine and the private
  networks, as for `http.response.fetch`.
- A made pipeline may use the delegated credentials and the ones its door made.

## 5. The same door as a published MCP tool

```
[t] trigger.mcp --route /admin --name zebflow_command --description "Run commands in the app's lab." --parameter commands:array! "The commands, each a tool and its arguments." --auth oauth --credential app_session --login /auth/login --role admin
[c] zebflow.command.run --credential zf_modules --command "{{ input.mcp.arguments.commands }}" --as "{{ input.mcp.auth.sub }}"
[ok] web.response.send --body "{{ input.command }}"
[no] web.response.send --status 403 --body "{{ input.command }}"
[t] -> [c]
[c] -> [ok]
[c]:error -> [no]
```

`input.mcp.auth` is the caller's verified token (`sub`, `roles`), so `--as`
names the person. An outside agent connects to `/_mcp/admin` on the app's host (the `mcp`
surface must be on, `help("guide/agentic/mcp-by-project")`); the fence holds exactly as it does
behind the webhook.

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
