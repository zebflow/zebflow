//! Concrete framework engine implementations.

pub mod basic;
mod noop;
pub mod wasm_host;

pub use basic::BasicPipelineEngine;
pub use basic::build_composite_placeholder_map;
pub use noop::NoopPipelineEngine;
pub use wasm_host::run_wasm_export;
