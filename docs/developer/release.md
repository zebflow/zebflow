# Release

A release changes the runtime. It must not silently change the meaning of an
existing project.

## Release Steps

1. Classify the change as patch, minor, or major.
2. List every public format and stored state it touches.
3. Run compatibility preflight on representative projects.
4. Run focused, integration, browser, and recovery tests.
5. Build all release files from one version value.
6. Publish crates, npm, pip, container, and release files as required.
7. Install the published release in a clean environment.
8. Verify update and rollback on a copy of real project data.

## Version Rules

A patch fixes behavior or performance without changing valid formats. A minor
release adds compatible behavior. A major release may change a stable contract
only with preflight, migration, backup, and a clear refusal path.

## Related Source

- `src/version.rs`
- `Cargo.toml`
- `npm/`
- `pip/`
- `docker/`
- `charts/`
- `.github/workflows/`
- `docs/contracts/versioning.md`
