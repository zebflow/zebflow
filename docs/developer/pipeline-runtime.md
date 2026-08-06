# Pipeline Runtime

Pipeline language belongs to users. Pipeline runtime belongs here.

Authority:

- `src/pipeline/model.rs`
- `src/pipeline/engines/basic.rs`
- `src/pipeline/expr/`
- `src/pipeline/nodes/basic/`
- `src/platform/services/pipeline_runtime.rs`
- `src/platform/services/pipeline_hits.rs`

Runtime responsibilities:

- parse DSL and JSON graph
- validate node kinds, pins, schemas, trigger rules, and config
- activate pipeline snapshots
- execute graph edges
- provide `input` and `ctx`
- call native, composite, and WASM node implementations
- store invocation history according to project policy
- enforce payload and trace behavior

Design principle:

Pipelines should behave like visual Rust. Data should flow only where dependencies require it. Large values should use files, references, tables, or storage-backed handles instead of being cloned through every node.
