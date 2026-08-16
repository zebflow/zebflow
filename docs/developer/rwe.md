# RWE

RWE compiles project TSX, renders the first HTML on the server, and adds browser
behavior. Zeb Tailwind scans supported classes and creates CSS.

## Main Parts

- `core/` parses, compiles, secures, and renders templates.
- `engines/` implements the public reactive web engine interface.
- `processors/` handles Markdown and Zeb Tailwind.
- `runtime/` contains browser and server runtime support files.
- `script_cache.rs` keeps compiled script references.
- `protocol.rs` defines compile and render request formats.

## Change Rules

- Keep TSX source valid and understandable.
- Use one shared component behavior across platform pages.
- Keep server and browser render output aligned.
- Treat supported class names as a public language.
- Add a compatibility test when adding a Tailwind pattern.
- Enforce import, file, and network boundaries in Rust.
- Report compiler errors with the source file and useful position.

## Related Source

- `src/rwe/mod.rs`
- `src/rwe/core/`
- `src/rwe/engines/`
- `src/rwe/processors/`
- `src/rwe/runtime/`
- `src/rwe/protocol.rs`
- `src/rwe/script_cache.rs`
- `libraries/`

## Dependency Boundary

The stable paths are local project TypeScript, reviewed Hub scripts, and
embedded Zeb Libraries. RWE resolves these inputs but does not act as an npm
client or package manager.

The removed npm preparation scaffold depended on external `npm` and `tar`
executables, created project `node_modules` links, wrote a second lock file, and
ran automatically after template saves. Those behaviors conflict with the
single-binary and frozen `DependencyLock` contracts.

Direct npm ingestion remains future work. It must not return until Zebflow can
resolve the full transitive graph, reject lifecycle scripts, limit archive
extraction, compile an approved browser artifact, record immutable digests, and
show a policy review before writing project files.
