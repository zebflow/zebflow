# Invocation log capture

`trace_capture.rs` owns log projection and capture budgets. `basic.rs` snapshots
project configuration at invocation start, resolves pipeline overrides, and
shares a `TraceCapture` budget across that run's node executions. Execution
payloads retain their existing ownership and node-to-node behavior.

Configure project defaults in **Settings → Logs**, and individual overrides in
**Pipeline Settings → Log data capture**. Apply pipeline changes to the draft,
save, and activate it for active-trigger execution. Changes affect future runs;
already persisted logs are not rewritten.

| Field | Default | Meaning |
| --- | ---: | --- |
| `array_sample_count` | 6 | Keep the first N elements of each array; 0 keeps all, subject to other limits. |
| `max_string_chars` | 8192 | Retained Unicode characters per string. |
| `max_depth` | 8 | Container depth from payload root at 0. |
| `max_node_bytes` | 65536 | Shared captured config/input/output JSON allowance per node execution. |
| `max_run_bytes` | 1048576 | Shared captured-data allowance per invocation. |

Missing fields inherit independently from project settings, then built-in
defaults. Explicit zero is only allowed for array sampling. Capture limits are
independent of existing count/age retention.

Sample wrappers add JSON nesting. A fixed encoded-depth ceiling of 112 reserves
room for invocation/API envelopes under the standard JSON parser limit, even
when `max_depth` is configured to 64. Extreme nesting can therefore be summarized
earlier than its configured payload-depth limit.

## Capture path

1. Scan borrowed input for private redaction markers, including omitted items.
   Only top-level redaction-exception paths are honored. The scan does not clone
   the payload, but remains linear in its size.
2. Serialize a borrowed view that samples arrays, limits nesting/strings, and
   masks secrets. Secret matching reads the original string so shortening cannot
   expose a token cut at a preview boundary. Overlapping declared secrets are
   masked as one range regardless of token order (for example `abc` and `bcdef`
   hide all of `abcdef`). Sensitive ancestors stay masked.
3. Stop serialization when its capped byte buffer would overflow. Replace that
   entire capture with a budget marker and consume its attempted allowance.
   Later captures in the same exhausted node/run also return budget markers.
4. Decode the compact buffer into the existing trace JSON type. Existing
   invocation persistence stores this compact trace, not the complete payload.

Budget consumption is ordered: config, input, output. A large config can use a
node's allowance before its input/output are captured. Budget markers and trace
bookkeeping (node IDs, timings, error messages) are outside these data budgets;
the limits are **not a hard cap on the whole SQLite invocation row**. Child
function invocations currently have their own independent budgets.

Truncation uses reserved structured `__zf_trace_summary` objects containing
type, original count, preview and/or reason. The inspector renders these as
unquoted ellipses. Actual user strings such as `"..."` remain quoted. Multiple
emissions preserve the existing `count`/`emissions` envelope; the emissions array
itself is also sampled. These projections must never be fed into execution.

## Verification and performance

Run `cargo test --lib trace_capture` for capture/contract/engine regressions and
`cargo test --lib trace_capture_benchmark -- --ignored --nocapture` for a local
capture-only comparison with the previous clone/redact/summarize implementation.
The legacy implementation is test-only and is not shipped in the runtime path.
It retains the old clone/redact/summarize structure but now shares the corrected
literal-secret matcher with execution and capture; it is not a frozen copy of
the old sequential replacement bug. Historical benchmark source hashes identify
the exact implementation used for earlier measurements.
The benchmark includes a disabled-capture baseline, small payloads, 10,000-row
payloads, and 30 repeated node captures. It reports time and serialized bytes.
Use `--release` for optimized measurements; debug timings are diagnostic only.

This phase reduces trace work. It does not introduce shared execution payloads,
linked function-run budgets, asynchronous persistence, or streaming nodes.
Existing node-to-node cloning and Rust/JavaScript serialization still occur.
