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

pub use engines::{BasicFrameworkEngine, NoopFrameworkEngine};
pub use interface::FrameworkEngine;
pub use model::{
    ExecuteOptions, FrameworkContext, FrameworkError, FrameworkOutput, NodeAiToolDefinition,
    NodeContractDocument, NodeContractItem, NodeDefinition, NodeScriptBridge,
    NodeScriptUsageContract, NodeToolUsageContract, NodeUsageMatrix, PipelineEdge, PipelineGraph,
    PipelineNode, StepEvent,
};
pub use nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput};
pub use registry::FrameworkEngineRegistry;
