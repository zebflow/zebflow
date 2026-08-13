# Repository Structure

This page tells a platform developer where each part of Zebflow lives.

## Top Level

```text
zebflow/
├── .github/       GitHub issue, release, and build automation
├── charts/        Helm chart for Kubernetes installation
├── composites/    composite node packages shipped with Zebflow
├── docker/        container build files and container helpers
├── docs/          stable user, developer, contract, and reference knowledge
├── integrations/  files for named external integrations
├── k8s/           Kubernetes examples and deployment resources
├── libraries/     Zeb React and RWE libraries available to projects
├── npm/           npm package that installs or runs Zebflow
├── pip/           Python package that installs or runs Zebflow
├── runtime/       runtime support files that are not Rust source
├── skills/        task guidance for working with Zebflow repositories
├── src/           Rust platform and runtime source
└── tests/         tests and fixtures that cross module boundaries
```

Local build output, runtime data, screenshots, and working notes do not belong
in this list. They are ignored by Git.

## Source Tree

```text
src/
├── lib.rs
├── version.rs
├── bin/
│   ├── zebflow.rs
│   ├── zebtune.rs
│   └── axum_rwe_demo.rs
├── automaton/
│   ├── agents/
│   │   └── engines/
│   ├── infra/
│   ├── intelligence/
│   ├── memory/
│   │   └── basic/
│   └── planning/
│       └── basic/
├── infra/
│   ├── cluster/
│   │   ├── config/
│   │   ├── k8s/
│   │   ├── registry/
│   │   ├── security/
│   │   └── transport/
│   ├── execution/
│   │   ├── backend/
│   │   ├── handle/
│   │   ├── placement/
│   │   ├── runner/
│   │   └── sync/
│   ├── io/
│   │   ├── cache/
│   │   ├── catalog/
│   │   ├── object/
│   │   ├── runtime_data/
│   │   └── state/
│   ├── mem/
│   ├── scheduler/
│   ├── storage/
│   ├── transport/
│   │   └── ws/
│   └── ws_client/
├── language/
│   ├── engines/
│   │   └── deno_sandbox/
│   └── runtime/
├── mapserver/
│   ├── infra/
│   │   └── source/
│   ├── publish/
│   └── resolve/
├── pipeline/
│   ├── engines/
│   ├── expr/
│   ├── nodes/
│   │   └── basic/
│   │       ├── logic/
│   │       ├── trigger/
│   │       └── web_response/
│   └── prototypes/
├── platform/
│   ├── adapters/
│   │   ├── data/
│   │   ├── file/
│   │   └── project_data/
│   ├── catalog/
│   │   └── ui/
│   ├── db/
│   │   └── drivers/
│   │       ├── postgresql/
│   │       └── sekejap/
│   ├── help/
│   │   ├── db/
│   │   ├── guide/
│   │   │   ├── agentic/
│   │   │   └── hub/
│   │   ├── pipeline/
│   │   │   └── examples/
│   │   ├── platform/
│   │   ├── tool/
│   │   └── web/
│   ├── interaction/
│   ├── mcp/
│   ├── policy/
│   ├── services/
│   │   ├── access/
│   │   └── cluster/
│   ├── shell/
│   └── web/
│       ├── assets/
│       │   ├── branding/
│       │   └── node-icons/
│       │       ├── brands/
│       │       └── zebflow/
│       └── templates/
│           ├── components/
│           │   └── ui/
│           ├── pages/
│           │   ├── dev/
│           │   │   └── design-system/
│           │   ├── home/
│           │   │   ├── components/
│           │   │   ├── marketplace/
│           │   │   └── project-templates/
│           │   ├── hub/
│           │   ├── login/
│           │   ├── profile/
│           │   ├── project-studio/
│           │   │   ├── components/
│           │   │   ├── connections/
│           │   │   │   └── db/
│           │   │   │       ├── connection/
│           │   │   │       ├── mapserver/
│           │   │   │       ├── postgresql/
│           │   │   │       └── sekejap/
│           │   │   ├── credentials/
│           │   │   ├── dashboard/
│           │   │   ├── files/
│           │   │   ├── hub/
│           │   │   ├── infrastructure/
│           │   │   │   └── components/
│           │   │   ├── pipelines/
│           │   │   │   └── registry/
│           │   │   │       └── components/
│           │   │   │           ├── nodes/
│           │   │   │           └── pipeline-editor/
│           │   │   │               ├── dialogs/
│           │   │   │               └── nodes/
│           │   │   └── settings/
│           │   │       └── clone/
│           │   │           └── ui/
│           │   │               └── preview/
│           │   └── test/
│           └── styles/
├── provision/
├── rwe/
│   ├── bench-fixtures/
│   ├── core/
│   ├── demo/
│   │   ├── showcase/
│   │   └── templates/
│   │       ├── components/
│   │       └── pages/
│   ├── engines/
│   ├── fixtures/
│   │   └── dx_test/
│   │       └── components/
│   ├── processors/
│   │   ├── markdown/
│   │   └── tailwind/
│   └── runtime/
└── zebfs/
```

## Source Responsibilities

- `src/lib.rs` lists public modules and creates the default engine kit.
- `src/version.rs` stores the runtime version value.
- `src/bin/` contains executable entry points and CLI behavior.
- `src/automaton/` contains agent planning, tools, memory, and model clients.
- `src/infra/` contains shared execution, storage, state, cluster, health, and
  transport parts.
- `src/language/` contains sandboxed user script engines and script contracts.
- `src/mapserver/` contains spatial publish and request handling.
- `src/pipeline/` contains the graph model, validation, expressions, node
  dispatch, and execution.
- `src/platform/` contains users, projects, auth, UI, APIs, MCP, Hub, databases,
  and services.
- `src/provision/` contains file based deployment and Kubernetes helpers.
- `src/rwe/` contains TSX compile, server render, browser behavior, and Zeb
  Tailwind.
- `src/zebfs/` contains the project file object model, local backend, and access
  rules.

## Finding the Owner

Start at the nearest `mod.rs`. It should explain the folder's job and list its
children. Follow the public types from that file before editing a leaf module.

If a feature crosses several folders, keep each concern with its owner. For
example, a file node belongs in `pipeline/nodes/`, file storage belongs in
`zebfs/`, and the file browser belongs in `platform/web/`.
