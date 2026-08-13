# Architecture

Zebflow is one Rust crate with modules that can be understood and tested on
their own. The executable wires them together.

## Main Flow

```text
HTTP or MCP
    -> platform service
    -> project source or active pipeline
    -> pipeline engine
    -> node handler
    -> database, file, map, script, web, or external service
```

RWE handles TSX pages. The pipeline engine handles graph work. ZebFS handles
project files. Mapserver handles spatial requests. Platform services connect
these parts to users and projects.

## Main Rules

- Keep public project behavior separate from internal Rust layout.
- Keep reusable runtime mechanics in `infra/`.
- Keep project and product behavior in `platform/`, `pipeline/`, `rwe/`,
  `mapserver/`, or `zebfs/`.
- Put validation close to the format or type it protects.
- Keep one source of truth for each schema.
- Make standalone mode a full supported mode.
- Add remote execution through clear interfaces, not branches in every node.
- Keep large data in storage and pass bounded values or references.

## Composition Root

`src/lib.rs` exposes the main modules and creates the default engine registries.
`src/bin/zebflow.rs` loads settings, chooses standalone, controller, or office
mode, builds the platform router, and starts the server.

## Related Source

- `src/lib.rs`
- `src/bin/zebflow.rs`
- `src/platform/mod.rs`
- `src/pipeline/mod.rs`
- `src/rwe/mod.rs`
- `src/infra/mod.rs`
- `src/language/mod.rs`
- `src/mapserver/mod.rs`
- `src/zebfs/mod.rs`
