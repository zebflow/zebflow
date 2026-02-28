# Test Layout

This crate uses integration tests grouped by domain:

- `tests/framework/`
- `tests/language/`
- `tests/`
- `tests/rwe/`

Each domain has a top-level runner file:

- `tests/framework.rs`
- `tests/language.rs`
- `tests/platform.rs`
- `tests/rwe.rs`

This makes it easy to grow tests by module without flattening everything into a single folder.
