# App Control

Status: **candidate**, drawn 2026-11-19. Node released in 0.11.31.

How an app built on Zebflow changes its own project while it runs: its
admins, or its own agent, send commands through a door, and Zebflow carries
them out inside a fence. The app's main code, the project's shared stores and
every credential not handed to the door stay out of reach.

```text
app admin or agent ─▶ door pipeline (trigger.webhook or trigger.mcp, with auth)
                       └▶ zebflow.command.run --credential <fence>
                            ├─ inside the fence ─▶ done, answered
                            └─ outside ──────────▶ refused with the reason
```

## The parts

| Part | What it is | Who changes it |
| --- | --- | --- |
| Door | any pipeline whose node runs `zebflow.command.run`; its trigger's `--auth` decides who may knock | a developer, through the project MCP or Studio |
| Fence | a credential of kind `zebflow_control` (`kinds/credential/README.md` § The `zebflow_control` fence) | a maintainer only; nothing sent through a door can change a fence |
| Made pipeline | a pipeline registered through a door | the door, inside the fence |
| Managed credential | a credential created through a door | that door only; a maintainer always |

## Fence fields

| Field | Meaning |
| --- | --- |
| `scope` | the only repository folder files and pipelines are written in |
| `routes` | the only path prefix a made webhook answers under (a path, not a host: a host serves only the prefixes its routes map, `addressing.md`) |
| `tools` | the operations the door opens, from the list below |
| `delegate` | credentials a maintainer hands to the door, by id |
| `nodes_mode`, `nodes` | `deny` (default; empty list = the shared stores `sekejap`, `sqlite`, `kv`, `fs`, `table`, `function`) or `allow` (only those listed, triggers included) |
| `manage_kinds` | credential kinds the door may create; empty = none |
| `manage_limit` | optional cap on how many credentials the door manages at once |
| `record_as` | how the change record names the door |

## Tools

`file_list`, `file_read`, `file_write`, `file_edit`, `file_delete`,
`file_stat`, `template_check` (amendment 2026-11-21: `file_stat` answers a file's `hash`; `file_write`, `file_edit` and
`pipeline_register` take `expect_hash` and refuse when the file changed since; `template_check` compiles a page in the folder without saving),
`pipeline_get`, `pipeline_register`, `pipeline_activate`,
`pipeline_deactivate`, `pipeline_delete`, `credential_list`,
`credential_create`, `credential_update`, `credential_delete`.

A command is `{ tool, arguments }`; a run takes 1 to 50, in order, and stops
at the first refusal; the ones before it stay done. The answer is
`command: { ok, results: [{ tool, ok, answer }] }`.

## Rules

1. Files and pipelines live only under `scope`. A path with `..`, a leading `/` or a backslash is refused, not repaired; no link inside `scope` is followed out of it; `scope` itself cannot be deleted.
2. Pipeline files (`.zf.json`) change only through the pipeline tools.
3. A made pipeline is checked from its body, again from the saved file (removed if it fails), and again when switched on. Every node is a catalogue kind exactly as written; it starts with `trigger.webhook`, `trigger.schedule`, `trigger.manual` or `trigger.repo`; it never uses `zebflow.*`; a webhook route lies under `routes` and carries no `%`, `\`, `?`, `#`, space or `.`/`..` segment; a `trigger.repo` watches a path inside `scope`, never the whole repository (amendment 2026-11-21).
4. A made pipeline names credentials written out by id, never by expression, and only those in `delegate` or managed by its own door.
5. A door creates only kinds in `manage_kinds`, never `zebflow_control`, and no more than `manage_limit`. A created credential is marked as managed by its door's fence in the credential record, not by its name.
6. `credential_list` answers the delegated and the managed credentials: id, kind, title and the non-secret fields. A secret value is never answered.
7. `credential_update` and `credential_delete` touch only managed credentials. A delegated one is read-only to the door.
8. Every address a created credential connects to must resolve outside the machine and the private networks (the rule `http.response.fetch` follows), checked at create, at update, and when it connects.
9. Every change is in the change record (`project.md` § Change record) as `control · <record_as>`, with the person from `--as`.

## Implementation

`src/pipeline/nodes/basic/zebflow/command.rs`; the fence kind in
`src/platform/services/credential.rs`. Tests:
`tests/platform/zebflow_command.rs` and the node's unit tests. Help:
`help("pipeline/examples/app-control-door")`.
