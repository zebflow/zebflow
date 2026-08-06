# Runtime Workflows

Common local checks:

```bash
cargo check
cargo fmt --check
cargo test --lib
```

Use narrower tests when the change is narrow.

Common local run:

```bash
zebflow
```

Default local URL:

```text
http://localhost:10610/login
```

If a running instance is needed, report the exact URL and port.

When reinstalling locally:

- confirm which binary or package path is used
- avoid killing unrelated user processes
- stop any test server started for the task before final response unless the user wants it running

When a task only changes docs, do not run a full Rust build unless the user asks or the docs are generated from code.
