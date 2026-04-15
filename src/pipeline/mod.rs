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
pub mod expr;
pub mod interface;
pub mod model;
pub mod nodes;
pub mod registry;

pub use engines::{BasicPipelineEngine, NoopPipelineEngine};
pub use interface::PipelineEngine;
pub use model::{
    ExecuteOptions, NodeAiToolDefinition, NodeContractDocument, NodeContractItem, NodeDefinition,
    NodeFieldDataSource, NodeFieldDef, NodeFieldType, NodeScriptBridge, NodeScriptUsageContract,
    NodeToolUsageContract, NodeUsageMatrix, PipelineContext, PipelineEdge, PipelineError,
    PipelineGraph, PipelineNode, PipelineOutput, SelectOptionDef, SidebarItem, SidebarSection,
    StepEvent,
};
pub use nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
pub use registry::PipelineEngineRegistry;
