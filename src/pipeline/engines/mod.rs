//! Concrete framework engine implementations.

pub mod basic;
mod noop;

pub use basic::BasicPipelineEngine;
pub use noop::NoopPipelineEngine;
