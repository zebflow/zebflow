//! Framework module for pipeline orchestration.
//!
//! Responsibility:
//!
//! - Validate pin-based graphs
//! - Execute graph traversal strategies
//! - Emit orchestration traces/errors
//!
//! This module does not embed script VM internals or template rendering logic.

pub mod engines;
pub mod interface;
pub mod model;
pub mod nodes;
pub mod registry;

pub use engines::NoopFrameworkEngine;
pub use interface::FrameworkEngine;
pub use model::{
    FrameworkContext, FrameworkError, FrameworkOutput, PipelineEdge, PipelineGraph, PipelineNode,
};
pub use nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput};
pub use registry::FrameworkEngineRegistry;
