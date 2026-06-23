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
pub mod layout;
pub mod model;
pub mod nodes;
pub mod prototypes;
pub mod registry;
pub mod security;

pub use engines::{BasicPipelineEngine, NoopPipelineEngine, build_composite_placeholder_map};
pub use interface::PipelineEngine;
pub use layout::auto_tidy_pipeline_graph;
pub use model::{
    ExecuteOptions, ExecutionBus, NodeAiToolDefinition, NodeContractDocument, NodeContractItem,
    NodeCredentialRequirement, NodeDefinition, NodeFieldDataSource, NodeFieldDef, NodeFieldType,
    NodeScriptBridge, NodeScriptUsageContract, NodeToolUsageContract, NodeUsageMatrix,
    PipelineContext, PipelineEdge, PipelineError, PipelineGraph, PipelineNode, PipelineOutput,
    SelectOptionDef, SidebarItem, SidebarSection, Signal, StepEvent,
};
pub use nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
pub use registry::PipelineEngineRegistry;
