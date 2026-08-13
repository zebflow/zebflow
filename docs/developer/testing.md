# Testing

Tests should match the risk of the change. A parser fix needs malformed input
tests. A storage change needs restart and recovery tests. A UI change needs a
real browser check.

## Test Levels

- Unit tests check one function or type.
- Module tests check one subsystem through its public interface.
- Integration tests check several modules together.
- Browser tests check visible user flows.
- Compatibility tests open existing project and package formats.
- Load tests measure time, memory, load control, and recovery.

## Required Checks

Before reporting a change complete:

1. Format changed Rust code.
2. Run focused tests for the changed module.
3. Run `cargo check` for the crate.
4. Test invalid input and partial failure.
5. Test the real UI when the change is visible.
6. Check that unrelated dirty files were not changed.
7. Record any test that could not run.

Memory tests must record baseline, peak, final idle memory, request latency, and
error count. A passing result that leaves the process growing without a bound is
not a complete pass.

## Related Source

- inline Rust test modules under `src/`
- `tests/`
- `src/rwe/fixtures/`
- `src/rwe/bench-fixtures/`
- `.github/workflows/`
