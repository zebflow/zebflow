# Runtime Workflows

Common local checks, cheapest first:

```bash
cargo test --test rwe platform_templates_parse   # < 1 s: every platform template parses, no borrowed bindings
cargo check
cargo test --test framework help_matches         # the help tree matches the code (DSL fences build, imports resolve)
cargo test --lib
cd tests/e2e && npm test                         # after any Studio or RWE change; fails on a console error
```

Use narrower tests when the change is narrow.

Common local run:

```bash
./dev.sh            # kills port 10610, cargo run, ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=admin123, data in .zebflow-platform-data
./dev.sh 10620      # a second, isolated instance
```

Platform templates are compiled into the binary: an edit under
`src/platform/web/templates/` shows nothing until `./dev.sh` rebuilds.

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
