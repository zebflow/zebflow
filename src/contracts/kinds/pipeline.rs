//! Permanent pipeline source contract.
//!
//! The structures in this module own the persisted `.zf.json` format. Runtime
//! execution continues to use [`PipelineGraph`], with explicit conversions at
//! this boundary, so engine refactoring cannot silently alter repository files.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::contracts::{
    CONTRACT_API_VERSION, ContractDocument, ContractError, ContractKind, ContractMetadata,
    PlatformContract, decode_contract, encode_contract,
};
use crate::pipeline::model::{
    PipelineGraphMetadata, PipelineGraphSettings, PipelineInvocationRetention,
};
use crate::pipeline::{PipelineEdge, PipelineGraph, PipelineNode};

/// Maximum accepted size of one persisted pipeline source document.
pub const MAX_PIPELINE_SOURCE_BYTES: usize = 16 * 1024 * 1024;
/// Maximum number of node instances in one pipeline source.
pub const MAX_PIPELINE_NODES: usize = 10_000;
/// Maximum number of directed edges in one pipeline source.
pub const MAX_PIPELINE_EDGES: usize = 100_000;

/// Canonical persisted pipeline contract.
pub struct PipelineContract;

impl PlatformContract for PipelineContract {
    type Spec = PipelineSpec;
    const KIND: ContractKind = ContractKind::Pipeline;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != spec.id {
            return Err(ContractError::violation(
                "FW_PIPELINE_ID",
                format!(
                    "metadata.name '{}' must match pipeline spec.id '{}'",
                    metadata.name, spec.id
                ),
            ));
        }
        if metadata.version.is_some()
            || metadata.digest.is_some()
            || !metadata.annotations.is_empty()
        {
            return Err(ContractError::violation(
                "FW_PIPELINE_METADATA",
                "Pipeline metadata contains only name",
            ));
        }
        spec.validate()
    }
}

/// Portable pipeline graph stored below the contract envelope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PipelineSpec {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<PipelineMetadataSpec>,
    #[serde(default)]
    pub entry_nodes: Vec<String>,
    pub nodes: Vec<PipelineNodeSpec>,
    pub edges: Vec<PipelineEdgeSpec>,
}

/// Mutable source controls that belong to one pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PipelineMetadataSpec {
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub settings: PipelineSettingsSpec,
}

/// Per-pipeline runtime policy overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PipelineSettingsSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_retention: Option<PipelineInvocationRetentionSpec>,
    /// Log preview limits, resolved field by field over project defaults.
    /// Array count zero retains all elements subject to independent byte/depth bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_capture: Option<crate::pipeline::trace_capture::TraceCaptureSettings>,
}

/// Optional invocation history bounds that override project defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PipelineInvocationRetentionSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_invocations: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_secs: Option<u64>,
}

/// One node instance in a persisted pipeline graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PipelineNodeSpec {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub input_pins: Vec<String>,
    #[serde(default)]
    pub output_pins: Vec<String>,
    #[serde(default = "empty_object")]
    pub config: Value,
}

/// One directed connection between two persisted node instances.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct PipelineEdgeSpec {
    pub from_node: String,
    pub from_pin: String,
    pub to_node: String,
    pub to_pin: String,
}

fn empty_object() -> Value {
    Value::Object(Map::new())
}

impl PipelineSpec {
    fn validate(&self) -> Result<(), ContractError> {
        validate_identifier("spec.id", &self.id, 128)?;
        if let Some(description) = &self.description {
            validate_text("spec.description", description, 16_384, true)?;
        }
        if self.nodes.len() > MAX_PIPELINE_NODES {
            return Err(ContractError::violation(
                "FW_PIPELINE_LIMIT",
                format!("spec.nodes exceeds the {MAX_PIPELINE_NODES} node limit"),
            ));
        }
        if self.edges.len() > MAX_PIPELINE_EDGES {
            return Err(ContractError::violation(
                "FW_PIPELINE_LIMIT",
                format!("spec.edges exceeds the {MAX_PIPELINE_EDGES} edge limit"),
            ));
        }

        if let Some(capture) = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.settings.trace_capture.as_ref())
        {
            capture.validate().map_err(|message| {
                ContractError::violation(
                    "FW_PIPELINE_TRACE_CAPTURE",
                    format!("spec.metadata.settings.trace_capture: {message}"),
                )
            })?;
        }

        if let Some(retention) = self
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.settings.invocation_retention.as_ref())
        {
            if retention.max_invocations.is_none() && retention.max_age_secs.is_none() {
                return Err(ContractError::violation(
                    "FW_PIPELINE_RETENTION",
                    "spec.metadata.settings.invocation_retention must set at least one limit",
                ));
            }
            if let Some(value) = retention.max_invocations
                && !(1..=1000).contains(&value)
            {
                return Err(ContractError::violation(
                    "FW_PIPELINE_RETENTION",
                    "spec.metadata.settings.invocation_retention.max_invocations must be between 1 and 1000",
                ));
            }
            if let Some(value) = retention.max_age_secs
                && !(1..=315_576_000).contains(&value)
            {
                return Err(ContractError::violation(
                    "FW_PIPELINE_RETENTION",
                    "spec.metadata.settings.invocation_retention.max_age_secs must be between 1 second and 10 years",
                ));
            }
        }

        if self.nodes.is_empty() {
            if self.entry_nodes.is_empty() && self.edges.is_empty() {
                return Ok(());
            }
            return Err(ContractError::violation(
                "FW_EMPTY_GRAPH",
                "an empty draft pipeline cannot contain entry nodes or edges",
            ));
        }

        let mut node_ids = HashSet::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            validate_identifier(&format!("spec.nodes[{index}].id"), &node.id, 128)?;
            validate_identifier(&format!("spec.nodes[{index}].kind"), &node.kind, 256)?;
            if !node_ids.insert(node.id.as_str()) {
                return Err(ContractError::violation(
                    "FW_DUPLICATE_NODE",
                    format!("spec.nodes contains duplicate node id '{}'", node.id),
                ));
            }
            if !node.config.is_object() {
                return Err(ContractError::violation(
                    "FW_NODE_CONFIG",
                    format!("spec.nodes[{index}].config must be an object"),
                ));
            }
            validate_pins(index, "input_pins", &node.input_pins)?;
            validate_pins(index, "output_pins", &node.output_pins)?;
        }

        let mut entries = HashSet::with_capacity(self.entry_nodes.len());
        for (index, entry) in self.entry_nodes.iter().enumerate() {
            validate_identifier(&format!("spec.entry_nodes[{index}]"), entry, 128)?;
            if !node_ids.contains(entry.as_str()) {
                return Err(ContractError::violation(
                    "FW_ENTRY_NODE",
                    format!("spec.entry_nodes[{index}] references unknown node '{entry}'"),
                ));
            }
            if !entries.insert(entry.as_str()) {
                return Err(ContractError::violation(
                    "FW_ENTRY_NODE",
                    format!("spec.entry_nodes contains duplicate node '{entry}'"),
                ));
            }
        }

        let nodes = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<std::collections::HashMap<_, _>>();
        let mut edges = HashSet::with_capacity(self.edges.len());
        for (index, edge) in self.edges.iter().enumerate() {
            let from = nodes.get(edge.from_node.as_str()).ok_or_else(|| {
                ContractError::violation(
                    "FW_EDGE_FROM_NODE",
                    format!(
                        "spec.edges[{index}].from_node references unknown node '{}'",
                        edge.from_node
                    ),
                )
            })?;
            let to = nodes.get(edge.to_node.as_str()).ok_or_else(|| {
                ContractError::violation(
                    "FW_EDGE_TO_NODE",
                    format!(
                        "spec.edges[{index}].to_node references unknown node '{}'",
                        edge.to_node
                    ),
                )
            })?;
            validate_identifier(
                &format!("spec.edges[{index}].from_pin"),
                &edge.from_pin,
                128,
            )?;
            validate_identifier(&format!("spec.edges[{index}].to_pin"), &edge.to_pin, 128)?;
            if edge.from_pin != "error" && !from.output_pins.contains(&edge.from_pin) {
                return Err(ContractError::violation(
                    "FW_EDGE_FROM_PIN",
                    format!(
                        "spec.edges[{index}].from_pin '{}' is not declared by node '{}'",
                        edge.from_pin, edge.from_node
                    ),
                ));
            }
            if !to.input_pins.contains(&edge.to_pin) {
                return Err(ContractError::violation(
                    "FW_EDGE_TO_PIN",
                    format!(
                        "spec.edges[{index}].to_pin '{}' is not declared by node '{}'",
                        edge.to_pin, edge.to_node
                    ),
                ));
            }
            if !edges.insert(edge) {
                return Err(ContractError::violation(
                    "FW_DUPLICATE_EDGE",
                    format!("spec.edges[{index}] duplicates an earlier edge"),
                ));
            }
        }
        Ok(())
    }
}

fn validate_pins(index: usize, field: &str, pins: &[String]) -> Result<(), ContractError> {
    let mut unique = HashSet::with_capacity(pins.len());
    for (pin_index, pin) in pins.iter().enumerate() {
        validate_identifier(
            &format!("spec.nodes[{index}].{field}[{pin_index}]"),
            pin,
            128,
        )?;
        if !unique.insert(pin.as_str()) {
            return Err(ContractError::violation(
                "FW_DUPLICATE_PIN",
                format!("spec.nodes[{index}].{field} contains duplicate pin '{pin}'"),
            ));
        }
    }
    Ok(())
}

fn validate_identifier(path: &str, value: &str, maximum: usize) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > maximum
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
    {
        return Err(ContractError::violation(
            "FW_PIPELINE_IDENTIFIER",
            format!(
                "{path} must contain only letters, numbers, dot, underscore, hyphen, and colon and be at most {maximum} bytes"
            ),
        ));
    }
    Ok(())
}

fn validate_text(
    path: &str,
    value: &str,
    maximum: usize,
    allow_newlines: bool,
) -> Result<(), ContractError> {
    if value.len() > maximum
        || value.chars().any(|character| {
            character == '\0'
                || (character.is_control()
                    && (!allow_newlines || !matches!(character, '\n' | '\r' | '\t')))
        })
    {
        return Err(ContractError::violation(
            "FW_PIPELINE_TEXT",
            format!(
                "{path} must be at most {maximum} bytes and contain no unsupported control characters"
            ),
        ));
    }
    Ok(())
}

impl From<PipelineSpec> for PipelineGraph {
    fn from(value: PipelineSpec) -> Self {
        Self {
            id: value.id,
            description: value.description,
            metadata: value.metadata.map(Into::into),
            entry_nodes: value.entry_nodes,
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            edges: value.edges.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PipelineGraph> for PipelineSpec {
    fn from(value: PipelineGraph) -> Self {
        Self {
            id: value.id,
            description: value.description,
            metadata: value.metadata.map(Into::into),
            entry_nodes: value.entry_nodes,
            nodes: value.nodes.into_iter().map(Into::into).collect(),
            edges: value.edges.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PipelineMetadataSpec> for PipelineGraphMetadata {
    fn from(value: PipelineMetadataSpec) -> Self {
        Self {
            locked: value.locked,
            settings: value.settings.into(),
        }
    }
}

impl From<PipelineGraphMetadata> for PipelineMetadataSpec {
    fn from(value: PipelineGraphMetadata) -> Self {
        Self {
            locked: value.locked,
            settings: value.settings.into(),
        }
    }
}

impl From<PipelineSettingsSpec> for PipelineGraphSettings {
    fn from(value: PipelineSettingsSpec) -> Self {
        Self {
            invocation_retention: value.invocation_retention.map(Into::into),
            trace_capture: value.trace_capture,
        }
    }
}

impl From<PipelineGraphSettings> for PipelineSettingsSpec {
    fn from(value: PipelineGraphSettings) -> Self {
        Self {
            invocation_retention: value.invocation_retention.map(Into::into),
            trace_capture: value.trace_capture,
        }
    }
}

impl From<PipelineInvocationRetentionSpec> for PipelineInvocationRetention {
    fn from(value: PipelineInvocationRetentionSpec) -> Self {
        Self {
            max_invocations: value.max_invocations,
            max_age_secs: value.max_age_secs,
        }
    }
}

impl From<PipelineInvocationRetention> for PipelineInvocationRetentionSpec {
    fn from(value: PipelineInvocationRetention) -> Self {
        Self {
            max_invocations: value.max_invocations,
            max_age_secs: value.max_age_secs,
        }
    }
}

impl From<PipelineNodeSpec> for PipelineNode {
    fn from(value: PipelineNodeSpec) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            input_pins: value.input_pins,
            output_pins: value.output_pins,
            config: value.config,
        }
    }
}

impl From<PipelineNode> for PipelineNodeSpec {
    fn from(value: PipelineNode) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            input_pins: value.input_pins,
            output_pins: value.output_pins,
            config: value.config,
        }
    }
}

impl From<PipelineEdgeSpec> for PipelineEdge {
    fn from(value: PipelineEdgeSpec) -> Self {
        Self {
            from_node: value.from_node,
            from_pin: value.from_pin,
            to_node: value.to_node,
            to_pin: value.to_pin,
        }
    }
}

impl From<PipelineEdge> for PipelineEdgeSpec {
    fn from(value: PipelineEdge) -> Self {
        Self {
            from_node: value.from_node,
            from_pin: value.from_pin,
            to_node: value.to_node,
            to_pin: value.to_pin,
        }
    }
}

/// Parses one canonical persisted pipeline document into the runtime graph.
pub fn decode_pipeline_graph(
    bytes: &[u8],
) -> Result<ContractDocument<PipelineGraph>, ContractError> {
    if bytes.len() > MAX_PIPELINE_SOURCE_BYTES {
        return Err(ContractError::violation(
            "FW_EMPTY_GRAPH",
            format!("Pipeline source exceeds the {MAX_PIPELINE_SOURCE_BYTES} byte limit"),
        ));
    }
    let document = decode_contract::<PipelineContract>(bytes)?;
    Ok(ContractDocument {
        api_version: CONTRACT_API_VERSION,
        kind: ContractKind::Pipeline.as_str(),
        metadata: document.metadata,
        spec: document.spec.into(),
    })
}

/// Validates and serializes one runtime graph as the canonical pipeline document.
pub fn encode_pipeline_graph(graph: PipelineGraph) -> Result<Vec<u8>, ContractError> {
    let metadata = ContractMetadata::named(graph.id.clone());
    let bytes = encode_contract::<PipelineContract>(metadata, graph.into())?;
    if bytes.len() > MAX_PIPELINE_SOURCE_BYTES {
        return Err(ContractError::invalid(format!(
            "Pipeline source exceeds the {MAX_PIPELINE_SOURCE_BYTES} byte limit"
        )));
    }
    Ok(bytes)
}

/// Validates one in-memory runtime graph against the frozen source contract.
///
/// Runtime engines use this for graphs constructed directly in memory. Graphs
/// decoded from `.zf.json` have already passed the same validation.
pub fn validate_pipeline_graph(graph: &PipelineGraph) -> Result<(), ContractError> {
    let metadata = ContractMetadata::named(graph.id.clone());
    PipelineContract::validate(&metadata, &graph.clone().into())
}

/// Requires a valid source graph to contain at least one executable node.
pub fn validate_pipeline_activation(graph: &PipelineGraph) -> Result<(), ContractError> {
    validate_pipeline_graph(graph)?;
    if graph.nodes.is_empty() {
        return Err(ContractError::invalid(format!(
            "pipeline '{}' has no nodes and cannot be activated",
            graph.id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_COMPLETE: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/pipeline/v1-complete.json");

    #[test]
    fn golden_v1_roundtrips_without_schema_drift() {
        let document = decode_pipeline_graph(V1_COMPLETE).expect("decode golden pipeline");
        let bytes = encode_pipeline_graph(document.spec.clone()).expect("encode pipeline");
        let encoded = decode_pipeline_graph(&bytes).expect("decode encoded pipeline");
        assert_eq!(document.metadata, encoded.metadata);
        assert_eq!(
            serde_json::to_value(document.spec).unwrap(),
            serde_json::to_value(encoded.spec).unwrap()
        );
    }

    #[test]
    fn trace_capture_overrides_roundtrip_and_reject_invalid_limits() {
        let mut value: Value = serde_json::from_slice(V1_COMPLETE).unwrap();
        let capture = serde_json::json!({
            "array_sample_count": 0,
            "max_string_chars": 1024,
            "max_depth": 4,
            "max_node_bytes": 4096,
            "max_run_bytes": 16384
        });
        value["spec"]["metadata"]["settings"]["trace_capture"] = capture.clone();
        let decoded = decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).unwrap();
        let encoded: Value =
            serde_json::from_slice(&encode_pipeline_graph(decoded.spec).unwrap()).unwrap();
        assert_eq!(
            encoded["spec"]["metadata"]["settings"]["trace_capture"],
            capture
        );

        for invalid in [
            serde_json::json!({"array_sample_count": -1}),
            serde_json::json!({"array_sample_count": 1.5}),
            serde_json::json!({"max_depth": 0}),
            serde_json::json!({"max_node_bytes": 0}),
            serde_json::json!({"max_run_bytes": 0}),
            serde_json::json!({"unknown": 1}),
        ] {
            value["spec"]["metadata"]["settings"]["trace_capture"] = invalid;
            assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());
        }
    }

    #[test]
    fn rejects_unknown_fields_and_release_metadata() {
        let mut value: Value = serde_json::from_slice(V1_COMPLETE).unwrap();
        value["spec"]["surprise"] = Value::Bool(true);
        assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value: Value = serde_json::from_slice(V1_COMPLETE).unwrap();
        value["metadata"]["version"] = Value::String("1.0.0".to_string());
        let error = decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(error.to_string().contains("contains only name"));
    }

    #[test]
    fn rejects_duplicate_nodes_pins_entries_and_edges() {
        let base: Value = serde_json::from_slice(V1_COMPLETE).unwrap();

        let mut value = base.clone();
        value["spec"]["nodes"][1]["id"] = Value::String("trigger".to_string());
        assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value = base.clone();
        value["spec"]["nodes"][1]["input_pins"] = serde_json::json!(["in", "in"]);
        assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value = base.clone();
        value["spec"]["entry_nodes"] = serde_json::json!(["trigger", "trigger"]);
        assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());

        let mut value = base;
        let edge = value["spec"]["edges"][0].clone();
        value["spec"]["edges"].as_array_mut().unwrap().push(edge);
        assert!(decode_pipeline_graph(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn empty_draft_is_valid_but_cannot_activate() {
        let source = br#"{
          "apiVersion":"zebflow.com/v1",
          "kind":"Pipeline",
          "metadata":{"name":"empty"},
          "spec":{"id":"empty","nodes":[],"edges":[]}
        }"#;
        let graph = decode_pipeline_graph(source).expect("empty draft").spec;
        assert!(validate_pipeline_activation(&graph).is_err());
    }

    #[test]
    fn omitted_node_config_has_frozen_empty_object_default() {
        let source = br#"{
          "apiVersion":"zebflow.com/v1",
          "kind":"Pipeline",
          "metadata":{"name":"default-config"},
          "spec":{"id":"default-config","nodes":[{
            "id":"trigger","kind":"n.trigger.manual","output_pins":["out"]
          }],"edges":[]}
        }"#;
        let graph = decode_pipeline_graph(source).expect("default config").spec;
        assert_eq!(graph.nodes[0].config, serde_json::json!({}));
    }
}
