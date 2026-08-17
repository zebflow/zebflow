# NodeBundle fixtures

Golden documents for the `zebflow.com/v1` `NodeBundle` contract. The round-trip
test in `src/contracts/kinds/node.rs` re-encodes each one and compares bytes, so
these files must stay in canonical encoder output form. Regenerate by decoding
and re-encoding through `decode_node_bundle` / `encode_node_bundle`, never by
hand-editing formatting.

| File | Covers |
| --- | --- |
| `v1-composite.json` | one composite action node with a scoped credential |
| `v1-wasm.json` | two WASM nodes sharing one module through distinct exports |
| `v1-mixed.json` | composite, WASM, and declarative trigger nodes in one bundle |
| `two-exports.wasm` | a real `zebflow-wasm-json-v1` module exporting `e2e_train` and `e2e_score` |

`two-exports.wasm` is a 22 KB Rust `cdylib` built for `wasm32-unknown-unknown`.
It exists so installation and runtime tests can prove that each WASM node runs
its own export rather than a shared default entry point. Its source is the
minimal ABI implementation described in the `NodeBundle` contract: exported
`memory`, `zebflow_alloc`, `zebflow_dealloc`, and one `(i32, i32) -> i64`
function per node that returns a packed pointer and length.
