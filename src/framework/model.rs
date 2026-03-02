//! Framework domain model for pipeline graph execution.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pipeline graph contract for framework orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    /// Graph contract kind marker.
    #[serde(default = "default_pipeline_kind")]
    pub kind: String,
    /// Graph contract version.
    #[serde(default = "default_pipeline_version")]
    pub version: String,
    /// Unique pipeline id.
    pub id: String,
    /// Node ids that can start execution.
    #[serde(default)]
    pub entry_nodes: Vec<String>,
    /// Node list.
    pub nodes: Vec<PipelineNode>,
    /// Directed pin edges.
    pub edges: Vec<PipelineEdge>,
}

/// Executable node definition in a pipeline graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    /// Unique node id in graph scope.
    pub id: String,
    /// Node kind id (for example `n.web.render`).
    pub kind: String,
    /// Input pin names.
    #[serde(default, alias = "inputs")]
    pub input_pins: Vec<String>,
    /// Output pin names.
    #[serde(default, alias = "outputs")]
    pub output_pins: Vec<String>,
    /// Node-specific configuration blob.
    #[serde(default)]
    pub config: Value,
}

/// Directed pin-like connection between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    /// Source node id.
    #[serde(alias = "from")]
    pub from_node: String,
    /// Source output pin.
    pub from_pin: String,
    /// Target node id.
    #[serde(alias = "to")]
    pub to_node: String,
    /// Target input pin.
    pub to_pin: String,
}

/// Script-bridge metadata exposed for node capabilities callable from `n.script`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NodeScriptBridge {
    /// Bridge function name under script namespace (for example `n.pg.query`).
    pub name: String,
    /// Whether this bridge is enabled in runtime.
    #[serde(default)]
    pub enabled: bool,
}

/// AI tool registration metadata for one node capability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeAiToolDefinition {
    /// Whether this node capability is exposed as an AI tool.
    #[serde(default)]
    pub registered: bool,
    /// Tool id or public name.
    #[serde(default)]
    pub tool_name: String,
    /// Human-readable tool description.
    #[serde(default)]
    pub tool_description: String,
    /// Tool input schema for LLM/tooling integration.
    #[serde(default)]
    pub tool_input_schema: Value,
}

/// Unified node definition contract used by runtime docs, UI, and tooling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeDefinition {
    /// Stable node kind id (for example `n.web.render`).
    pub kind: String,
    /// Display title for UI catalogs.
    pub title: String,
    /// Behavior description used by docs and UI.
    pub description: String,
    /// Input JSON schema.
    #[serde(default)]
    pub input_schema: Value,
    /// Output JSON schema.
    #[serde(default)]
    pub output_schema: Value,
    /// Declared input pins.
    #[serde(default)]
    pub input_pins: Vec<String>,
    /// Declared output pins.
    #[serde(default)]
    pub output_pins: Vec<String>,
    /// Whether capability is available from script runtime bridge.
    #[serde(default)]
    pub script_available: bool,
    /// Optional script bridge metadata.
    #[serde(default)]
    pub script_bridge: Option<NodeScriptBridge>,
    /// AI tool registration metadata.
    #[serde(default)]
    pub ai_tool: NodeAiToolDefinition,
}

/// Script bridge usage contract for one node kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeScriptUsageContract {
    /// Whether this node can be called from `n.script`.
    pub available: bool,
    /// Bridge function name exposed in script (for example `n.pg.query`).
    #[serde(default)]
    pub bridge_name: String,
    /// Whether the script bridge is enabled in runtime.
    #[serde(default)]
    pub enabled: bool,
}

/// AI tool usage contract for one node kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeToolUsageContract {
    /// Whether this node is registered as an AI tool.
    pub registered: bool,
    /// Tool name/id.
    #[serde(default)]
    pub tool_name: String,
    /// Tool description.
    #[serde(default)]
    pub tool_description: String,
    /// Tool input schema.
    #[serde(default)]
    pub tool_input_schema: Value,
}

/// Usage matrix showing where the node contract can be used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeUsageMatrix {
    /// Available as a pipeline node in graph execution.
    pub pipeline_node: bool,
    /// Script bridge contract.
    pub script_usage: NodeScriptUsageContract,
    /// AI tool contract.
    pub tool_usage: NodeToolUsageContract,
}

/// Extractable node contract item for `/docs/node`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeContractItem {
    /// Stable node kind id.
    pub kind: String,
    /// Display title.
    pub title: String,
    /// Human description.
    pub description: String,
    /// Input JSON schema.
    #[serde(default)]
    pub input_schema: Value,
    /// Output JSON schema.
    #[serde(default)]
    pub output_schema: Value,
    /// Input pins.
    #[serde(default)]
    pub input_pins: Vec<String>,
    /// Output pins.
    #[serde(default)]
    pub output_pins: Vec<String>,
    /// Usage matrix.
    pub usage_matrix: NodeUsageMatrix,
}

/// Root node contract document served at `/docs/node`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NodeContractDocument {
    /// Marker for successful extraction.
    pub ok: bool,
    /// Stable contract schema version.
    pub schema_version: &'static str,
    /// Source anchor for traceability.
    pub source: &'static str,
    /// Node contract entries.
    #[serde(default)]
    pub items: Vec<NodeContractItem>,
}

impl From<NodeDefinition> for NodeContractItem {
    fn from(value: NodeDefinition) -> Self {
        let (bridge_name, bridge_enabled) = value
            .script_bridge
            .as_ref()
            .map(|bridge| (bridge.name.clone(), bridge.enabled))
            .unwrap_or_else(|| (String::new(), false));
        Self {
            kind: value.kind,
            title: value.title,
            description: value.description,
            input_schema: value.input_schema,
            output_schema: value.output_schema,
            input_pins: value.input_pins,
            output_pins: value.output_pins,
            usage_matrix: NodeUsageMatrix {
                pipeline_node: true,
                script_usage: NodeScriptUsageContract {
                    available: value.script_available,
                    bridge_name,
                    enabled: bridge_enabled,
                },
                tool_usage: NodeToolUsageContract {
                    registered: value.ai_tool.registered,
                    tool_name: value.ai_tool.tool_name,
                    tool_description: value.ai_tool.tool_description,
                    tool_input_schema: value.ai_tool.tool_input_schema,
                },
            },
        }
    }
}

/// One step event for streaming (thinking, tool_call, tool_result, final, external, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEvent {
    pub step: String,
    pub description: String,
    pub at: String,
}

/// Options for execution (e.g. step stream for SSE).
#[derive(Debug, Default)]
pub struct ExecuteOptions {
    /// When set, nodes (e.g. Zebtune) send each step here for streaming. Consumer can forward to SSE.
    pub step_tx: Option<tokio::sync::mpsc::UnboundedSender<StepEvent>>,
}

/// Runtime context for a framework run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkContext {
    /// Owner/tenant id.
    pub owner: String,
    /// Project id.
    pub project: String,
    /// Pipeline id.
    pub pipeline: String,
    /// Request/run id.
    pub request_id: String,
    /// Trigger payload.
    pub input: Value,
}

/// Standard framework execution output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkOutput {
    /// Final output payload.
    pub value: Value,
    /// Ordered trace entries emitted by framework.
    pub trace: Vec<String>,
}

/// Framework layer error model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl FrameworkError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for FrameworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for FrameworkError {}

fn default_pipeline_kind() -> String {
    "zebflow.pipeline".to_string()
}

fn default_pipeline_version() -> String {
    "0.1".to_string()
}
