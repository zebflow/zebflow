---
name: zebflow-basic
description: How to work in any Zebflow project over MCP. Use at the start of every session and whenever you are about to create, change or claim completion of anything in a project — orientation, the mutation rule, exact names, verification, memory.
license: MIT
metadata:
  version: "1"
  audience: agents building Zebflow projects
---

# Zebflow — the way of working

A Zebflow project is pipelines plus the files they use, served by one binary.
There is no build step and no local checkout: you change the project through
the MCP tools, and the running instance is the only truth about whether it
worked.

## Start every session the same way

1. `start_here` — what the project has: pipelines and their status, files,
   connections, the skills you can load.
2. `docs_agent_read name="AGENTS.md"` — the project's own rules. They win over
   this skill and over the help.
3. `docs_agent_read name="MEMORY.md"` — what earlier sessions did and left
   open. Write your goal there before you start.
4. `file_read rel_path="docs/structure.md"` — where things go in this
   project. If it is missing and you are about to create files, read
   `zebflow-engineering` first: it chooses the layout and writes that file.
5. When the task names a domain you have not touched this session, read its
   skill (`skill_read name="zebflow-pipeline"`, `zebflow-rwe`, `zebflow-ui`,
   `zebflow-data`, `zebflow-auth`, `zebflow-files-editor`, `zebflow-hub`) and
   the help topic it points to. Skills say when and in what order; the help
   says how things are shaped.

## The rules that hold everywhere

- **Everything goes through the tools.** Pipelines through `pipeline_register`
  / `pipeline_patch` / `pipeline_activate`; files through `file_write` /
  `file_edit`; docs through `file_write rel_path="docs/…"`; agent docs through
  `docs_agent_write`. There is no other door, and guessing one by filesystem
  layout is a bug.
- **Read exact names before you use them.** A template path comes from
  `file_list` and ends in `.tsx`; a credential id from `credential_list`
  (never a connection slug); a table from `connection_describe`; a node's flags
  from `help(topic="pipeline/nodes/<kind>")`. A name from memory or from
  another project is a guess.
- **Draft is not live.** `pipeline_register` and `pipeline_patch` leave the
  pipeline `draft` or `stale`; nothing serves until `pipeline_activate`.
- **A 200 is not a page.** A component that throws is replaced by
  `<!-- RWE component error: … -->` and the response is still 200; a page
  whose hydration failed still serves correct HTML. Fetch and read the body;
  open it in a browser. `skill_read name="zebflow-verify"` has the checks.
- **Webhook data is under `input.body`.** A form field is `input.body.email`;
  the route's parameters are `input.params`, the query `input.query`. In `{{ }}`
  the request is `$trigger.params`, `$trigger.query`, `$trigger.auth` — there
  is no `$trigger.body`.
- **Every file imports what it uses** from `"zeb/react"`, `"zeb/ui/<name>"` or
  `"@/…"`. Nothing is inherited from the page that imports it.
- **Quote any flag value that contains `{{ }}` or a space** as one argument.
- **Locked means locked.** If a tool answers `PLATFORM_PIPELINE_LOCKED` or
  `PLATFORM_TEMPLATE_LOCKED`, stop and tell the user; you cannot unlock.
- **A refused capability is not a puzzle.** The session may be read-only or
  without git; say so instead of working around it.

## Before you say "done"

Done means witnessed on the running instance, not "the source looks right":

1. The pipeline is `active` in `pipeline_list` (not `draft`, not `stale`).
2. The route was fetched (`/wh/{owner}/{project}{path}`) and the body was read:
   no `RWE component error`, the data you expected is there, the status is
   the one you meant.
3. For a page, a browser opened it with no console error and the interaction
   you built produced the state it should (`zebflow-verify`).
4. For a failure you fixed, `pipeline_get_invocations` shows a clean run
   after the fix.
5. `MEMORY.md` says what was built, what was verified, and what is still open
   — in that order, briefly. Commit with `git_command` when the project uses
   git and AGENTS.md does not say otherwise.

If any of these cannot be done, say which one and why. That is a complete
answer; "it should work" is not.

## Where the facts are

| Question | Read |
|---|---|
| the DSL, nodes, responses | `help(topic="pipeline")`, `pipeline/dsl`, `pipeline/web`, `pipeline/nodes/<kind>` |
| pages, hooks, the UI kit, libraries | `help(topic="web")`, `web/hooks`, `web/ui`, `web/tailwind`, `web/libraries` |
| databases | `help(topic="db")`, `db/sekejap` |
| the platform, the API, operations | `help(topic="platform")`, `platform/api`, `platform/operations` |
| a full recipe | `help(topic="pipeline/examples")` |
| anything by keyword | `help_search query="…"` — searches the help and every node definition |
