# Using Zebflow

**How to build with Zebflow lives in the help tree**, `src/platform/help/`,
which the binary embeds. It is the same text an agent reads over MCP with
`help(topic)` and a person reads in the Studio's Help. There is one copy so it
cannot drift.

| Building… | Topic |
|---|---|
| pipelines, the DSL, responses | `pipeline`, `pipeline/dsl`, `pipeline/authoring`, `pipeline/web` |
| pages, components, the UI kit, libraries | `web`, `web/hooks`, `web/ui`, `web/tailwind`, `web/libraries` |
| databases | `db`, `db/sekejap` |
| script helpers | `tool` |
| end-to-end recipes | `pipeline/examples/*` |
| the platform, API, operations, agent workflow | `platform`, `platform/api`, `platform/operations`, `platform/workflow`, `platform/agent` |
| hub, mail, maps, federation, credentials | `guide/hub`, `guide/sending-mail`, `guide/mapserver`, `guide/federated-offices`, `guide/credential-encryption` |

Read them as files under [`src/platform/help/`](../../src/platform/help/), or
run Zebflow and open Help. The **skills** — the procedures an agent follows
for each kind of task — are under [`blessed/skills/`](../../blessed/skills/)
(the core set every project lists over MCP: `skill_list`, `skill_read`) and
[`blessed/skill-extras/`](../../blessed/skill-extras/) (optional, added from
the hub), and published
as `zebflow/skills` for agents that read skills from a folder
(`scripts/publish-skills.sh`).

This folder keeps only what an **operator** needs before there is a running
instance to ask:

1. [Install Zebflow](./installation.md)
2. [Deploy and operate it](./deployment.md)
3. [Author your own nodes](./nodes/README.md) — composite and WASM

The [reference](../reference/README.md) lists error-code families; every other
exact name (routes, tools, flags) is in the help tree or generated from the
code (`help(topic="pipeline/nodes")`).
