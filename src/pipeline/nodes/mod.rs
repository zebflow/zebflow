//! Framework node interfaces and built-in node implementations.

pub mod basic;
mod interface;

pub use interface::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput};

/// Returns all built-in node definitions.
pub fn builtin_node_definitions() -> Vec<crate::pipeline::NodeDefinition> {
    basic::builtin_node_definitions()
}
