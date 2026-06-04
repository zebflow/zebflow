//! Concrete framework engine implementations.

pub mod basic;
mod noop;

pub use basic::BasicPipelineEngine;
pub use basic::build_composite_placeholder_map;
pub use noop::NoopPipelineEngine;
