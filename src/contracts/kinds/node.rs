use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use std::collections::HashSet;
use std::path::{Component, Path};

use crate::pipeline::NodeDefinition;
use crate::platform::model::{MultiNodePackageDefinition, NodePackageManifest, NodePackageSource};

/// Maximum serialized size of one standalone normalized node definition.
pub const MAX_NODE_DEFINITION_BYTES: usize = 512 * 1024;

/// WASM ABI supported by the stable v1 node contract.
pub const WASM_JSON_ABI_V1: &str = "zebflow-wasm-json-v1";

/// Decode one bounded canonical NodeDefinition document.
pub fn decode_node_definition(
    bytes: &[u8],
) -> Result<crate::contracts::ContractDocument<NodeDefinition>, ContractError> {
    if bytes.len() > MAX_NODE_DEFINITION_BYTES {
        return Err(ContractError::invalid(format!(
            "NodeDefinition exceeds the {MAX_NODE_DEFINITION_BYTES} byte limit"
        )));
    }
    crate::contracts::decode_contract::<NodeDefinitionContract>(bytes)
}

/// Encode one bounded canonical NodeDefinition document.
pub fn encode_node_definition(
    metadata: ContractMetadata,
    definition: NodeDefinition,
) -> Result<Vec<u8>, ContractError> {
    let bytes = crate::contracts::encode_contract::<NodeDefinitionContract>(metadata, definition)?;
    if bytes.len() > MAX_NODE_DEFINITION_BYTES {
        return Err(ContractError::invalid(format!(
            "NodeDefinition exceeds the {MAX_NODE_DEFINITION_BYTES} byte limit"
        )));
    }
    Ok(bytes)
}

/// Canonical source package containing one or more installable nodes.
pub struct NodeBundleContract;

impl PlatformContract for NodeBundleContract {
    type Spec = MultiNodePackageDefinition;
    const KIND: ContractKind = ContractKind::NodeBundle;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        validate_release_metadata(metadata, &spec.package, &spec.version)?;
        validate_package_slug(&spec.package)?;
        if spec.title.trim().is_empty() || spec.description.trim().is_empty() {
            return Err(ContractError::invalid(
                "spec.title and spec.description must not be empty",
            ));
        }
        if !spec.icon.is_empty() {
            validate_relative_file("spec.icon", &spec.icon)?;
        }
        for (name, path) in &spec.functions {
            if name.trim().is_empty() {
                return Err(ContractError::invalid(
                    "spec.functions names must not be empty",
                ));
            }
            validate_relative_file(&format!("spec.functions.{name}"), path)?;
        }
        if spec.nodes.is_empty() {
            return Err(ContractError::invalid(
                "NodeBundle must contain at least one node",
            ));
        }
        let definitions = normalize_node_bundle(spec)?;
        let mut kinds = HashSet::new();
        for (entry, definition) in spec.nodes.iter().zip(definitions) {
            if !entry.icon.is_empty() {
                validate_relative_file(&format!("spec.nodes[{}].icon", entry.kind), &entry.icon)?;
            }
            if !kinds.insert(definition.definition.kind.clone()) {
                return Err(ContractError::invalid(format!(
                    "spec.nodes contains duplicate node kind '{}'",
                    definition.definition.kind
                )));
            }
            validate_node_definition_spec(&definition)?;
        }
        Ok(())
    }
}

fn validate_package_slug(value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(ContractError::invalid(
            "spec.package must be a lowercase alphanumeric slug with optional internal hyphens",
        ));
    }
    Ok(())
}

/// Canonical normalized contract for one node kind.
///
/// Native Rust, composite, and WASM nodes expose this same definition. Runtime
/// linkage and package release identity belong to [`NodeBundleContract`], not
/// to this implementation-neutral interface.
pub struct NodeDefinitionContract;

impl PlatformContract for NodeDefinitionContract {
    type Spec = NodeDefinition;
    const KIND: ContractKind = ContractKind::NodeDefinition;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != spec.kind {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match node kind '{}'",
                metadata.name, spec.kind
            )));
        }
        if metadata.version.is_some() {
            return Err(ContractError::invalid(
                "NodeDefinition metadata.version is forbidden; bundle releases own node versions",
            ));
        }
        validate_normalized_node_definition(spec)
    }
}

/// Convert a source bundle into the implementation-neutral per-node runtime form.
pub fn normalize_node_bundle(
    package: &MultiNodePackageDefinition,
) -> Result<Vec<NodePackageManifest>, ContractError> {
    let mut manifests = Vec::with_capacity(package.nodes.len());
    for node in &package.nodes {
        let is_wasm = node.kind.starts_with("n.wasm.");
        let definition = NodeDefinition {
            kind: node.kind.clone(),
            title: node.title.clone(),
            description: node.description.clone(),
            config_schema: node.definition.config_schema.clone(),
            input_schema: node.definition.input_schema.clone(),
            output_schema: node.definition.output_schema.clone(),
            examples: node.definition.examples.clone(),
            failure_semantics: node.definition.failure_semantics.clone(),
            input_pins: node.definition.input_pins.clone(),
            output_pins: node.definition.output_pins.clone(),
            dsl_flags: node.definition.dsl_flags.clone(),
            fields: node.definition.fields.clone(),
            layout: node.definition.layout.clone(),
            ui_category: node.ui_category.clone(),
            ui_category_label: node.ui_category_label.clone(),
            ..Default::default()
        };
        manifests.push(NodePackageManifest {
            source: if is_wasm {
                NodePackageSource::Wasm
            } else {
                NodePackageSource::Composite
            },
            version: package.version.clone(),
            definition,
            credentials: package.credentials.clone(),
            wasm_runtime: is_wasm.then(|| package.wasm_runtime.clone()).flatten(),
            functions: if is_wasm {
                Default::default()
            } else {
                package.functions.clone()
            },
            main_function: if is_wasm { None } else { node.main.clone() },
            trigger: if is_wasm { None } else { node.trigger.clone() },
            lifecycle: if is_wasm {
                None
            } else {
                node.lifecycle.clone()
            },
        });
    }
    Ok(manifests)
}

/// Validate the shared interface consumed by docs, UI, DSL, and runtime code.
pub fn validate_normalized_node_definition(
    definition: &NodeDefinition,
) -> Result<(), ContractError> {
    crate::pipeline::nodes::validate_node_definition_contract(definition).map_err(|errors| {
        ContractError::invalid(format!(
            "node definition is incomplete: {}",
            errors.join("; ")
        ))
    })?;
    validate_unique_tokens("spec.input_pins", &definition.input_pins)?;
    validate_unique_tokens("spec.output_pins", &definition.output_pins)?;
    validate_schema("spec.config_schema", &definition.config_schema)?;
    validate_schema("spec.input_schema", &definition.input_schema)?;
    validate_schema("spec.output_schema", &definition.output_schema)?;

    for (index, example) in definition.examples.iter().enumerate() {
        if example.title.trim().is_empty() {
            return Err(ContractError::invalid(format!(
                "spec.examples[{index}].title must not be empty"
            )));
        }
        validate_schema(&format!("spec.examples[{index}].config"), &example.config)?;
    }
    let mut failure_codes = HashSet::new();
    for (index, failure) in definition.failure_semantics.iter().enumerate() {
        if failure.code.trim().is_empty() || failure.description.trim().is_empty() {
            return Err(ContractError::invalid(format!(
                "spec.failure_semantics[{index}] requires code and description"
            )));
        }
        if !failure_codes.insert(failure.code.as_str()) {
            return Err(ContractError::invalid(format!(
                "spec.failure_semantics contains duplicate code '{}'",
                failure.code
            )));
        }
    }

    let kind_segments = definition.kind.split('.').collect::<Vec<_>>();
    if kind_segments.len() < 2
        || kind_segments[0] != "n"
        || kind_segments[1..].iter().any(|segment| {
            segment.is_empty()
                || !segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(ContractError::invalid(
            "spec.kind must use lowercase dot-separated n.* segments",
        ));
    }

    let mut flags = HashSet::new();
    for flag in &definition.dsl_flags {
        if !flags.insert(flag.flag.as_str()) {
            return Err(ContractError::invalid(format!(
                "spec.dsl_flags contains duplicate flag '{}'",
                flag.flag
            )));
        }
    }

    if definition.script_available != definition.script_bridge.is_some() {
        return Err(ContractError::invalid(
            "spec.script_available and spec.script_bridge must be declared together",
        ));
    }
    if let Some(bridge) = &definition.script_bridge
        && (bridge.name.trim().is_empty() || !bridge.name.starts_with("n."))
    {
        return Err(ContractError::invalid(
            "spec.script_bridge.name must be a non-empty n.* identifier",
        ));
    }
    if definition.ai_tool.registered {
        if definition.ai_tool.tool_name.trim().is_empty()
            || definition.ai_tool.tool_description.trim().is_empty()
        {
            return Err(ContractError::invalid(
                "registered spec.ai_tool requires tool_name and tool_description",
            ));
        }
        validate_schema(
            "spec.ai_tool.tool_input_schema",
            &definition.ai_tool.tool_input_schema,
        )?;
    } else if !definition.ai_tool.tool_name.is_empty()
        || !definition.ai_tool.tool_description.is_empty()
        || !definition.ai_tool.tool_input_schema.is_null()
    {
        return Err(ContractError::invalid(
            "unregistered spec.ai_tool must not declare tool metadata",
        ));
    }
    Ok(())
}

/// Validate all context-free rules of one durable installed node definition.
///
/// Project-specific collision checks remain the node registry's responsibility.
pub fn validate_node_definition_spec(spec: &NodePackageManifest) -> Result<(), ContractError> {
    validate_release_version(&spec.version)?;
    validate_normalized_node_definition(&spec.definition)?;
    validate_credentials(spec)?;

    match spec.source {
        NodePackageSource::Composite => validate_composite_runtime(spec),
        NodePackageSource::Wasm => validate_wasm_runtime(spec),
    }
}

fn validate_release_version(version: &str) -> Result<(), ContractError> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(ContractError::invalid(
            "spec.version must be a numeric major.minor.patch release",
        ));
    }
    Ok(())
}

fn validate_unique_tokens(path: &str, values: &[String]) -> Result<(), ContractError> {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(ContractError::invalid(format!(
                "{path} must not contain duplicate '{value}'"
            )));
        }
    }
    Ok(())
}

fn validate_schema(path: &str, value: &serde_json::Value) -> Result<(), ContractError> {
    if !value.is_null() && !value.is_object() {
        return Err(ContractError::invalid(format!(
            "{path} must be a JSON Schema object or null"
        )));
    }
    Ok(())
}

fn validate_credentials(spec: &NodePackageManifest) -> Result<(), ContractError> {
    let properties = spec
        .definition
        .config_schema
        .get("properties")
        .and_then(serde_json::Value::as_object);
    let mut kinds = HashSet::new();
    for credential in &spec.credentials {
        if credential.kind.trim().is_empty()
            || credential.title.trim().is_empty()
            || credential.description.trim().is_empty()
            || credential.config_key.trim().is_empty()
        {
            return Err(ContractError::invalid(
                "credential kind, title, description, and config_key must not be empty",
            ));
        }
        if !kinds.insert(&credential.kind) {
            return Err(ContractError::invalid(format!(
                "credential kind '{}' is declared more than once",
                credential.kind
            )));
        }
        if !properties.is_some_and(|values| values.contains_key(&credential.config_key)) {
            return Err(ContractError::invalid(format!(
                "credential '{}' config_key '{}' must exist in config_schema.properties",
                credential.kind, credential.config_key
            )));
        }
        if !spec
            .definition
            .fields
            .iter()
            .any(|field| field.name == credential.config_key)
            || !spec
                .definition
                .dsl_flags
                .iter()
                .any(|flag| flag.config_key == credential.config_key)
        {
            return Err(ContractError::invalid(format!(
                "credential '{}' config_key '{}' must be documented by fields and dsl_flags",
                credential.kind, credential.config_key
            )));
        }
    }
    Ok(())
}

fn validate_composite_runtime(spec: &NodePackageManifest) -> Result<(), ContractError> {
    let kind = &spec.definition.kind;
    if !kind.starts_with("n.c.") {
        return Err(ContractError::invalid(format!(
            "composite node kind '{kind}' must start with 'n.c.'"
        )));
    }
    if spec.wasm_runtime.is_some() {
        return Err(ContractError::invalid(
            "composite node must not declare spec.wasm_runtime",
        ));
    }
    if spec.main_function.is_none() && spec.trigger.is_none() {
        return Err(ContractError::invalid(
            "composite node must declare main_function or trigger",
        ));
    }
    for (name, path) in &spec.functions {
        if name.trim().is_empty() {
            return Err(ContractError::invalid(
                "spec.functions names must not be empty",
            ));
        }
        validate_relative_file(&format!("spec.functions.{name}"), path)?;
    }
    for (path, reference) in [
        ("spec.main_function", spec.main_function.as_ref()),
        (
            "spec.trigger.on_message",
            spec.trigger
                .as_ref()
                .and_then(|value| value.on_message.as_ref()),
        ),
        (
            "spec.lifecycle.on_activate",
            spec.lifecycle
                .as_ref()
                .and_then(|value| value.on_activate.as_ref()),
        ),
        (
            "spec.lifecycle.on_deactivate",
            spec.lifecycle
                .as_ref()
                .and_then(|value| value.on_deactivate.as_ref()),
        ),
    ] {
        if let Some(reference) = reference
            && !spec.functions.contains_key(reference)
        {
            return Err(ContractError::invalid(format!(
                "{path} references missing function '{reference}'"
            )));
        }
    }
    Ok(())
}

fn validate_wasm_runtime(spec: &NodePackageManifest) -> Result<(), ContractError> {
    let kind = &spec.definition.kind;
    if !kind.starts_with("n.wasm.") {
        return Err(ContractError::invalid(format!(
            "WASM node kind '{kind}' must start with 'n.wasm.'"
        )));
    }
    if !spec.functions.is_empty()
        || spec.main_function.is_some()
        || spec.trigger.is_some()
        || spec.lifecycle.is_some()
    {
        return Err(ContractError::invalid(
            "WASM node must not declare composite runtime fields",
        ));
    }
    let runtime = spec
        .wasm_runtime
        .as_ref()
        .ok_or_else(|| ContractError::invalid("WASM node must declare spec.wasm_runtime"))?;
    validate_relative_file("spec.wasm_runtime.module", &runtime.module)?;
    if runtime.abi != WASM_JSON_ABI_V1 {
        return Err(ContractError::invalid(format!(
            "spec.wasm_runtime.abi must be '{WASM_JSON_ABI_V1}'"
        )));
    }
    Ok(())
}

fn validate_relative_file(path: &str, value: &str) -> Result<(), ContractError> {
    let value_path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value_path.is_absolute()
        || value_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(ContractError::invalid(format!(
            "{path} must be a normalized relative file path"
        )));
    }
    Ok(())
}

fn validate_release_metadata(
    metadata: &ContractMetadata,
    expected_name: &str,
    expected_version: &str,
) -> Result<(), ContractError> {
    if metadata.name != expected_name {
        return Err(ContractError::invalid(format!(
            "metadata.name '{}' must match '{}'",
            metadata.name, expected_name
        )));
    }
    if metadata.version.as_deref() != Some(expected_version) {
        return Err(ContractError::invalid(format!(
            "metadata.version must match release version '{expected_version}'"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1_COMPLETE: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-definition/v1-complete.json");

    fn wasm_manifest() -> NodePackageManifest {
        serde_json::from_value(serde_json::json!({
            "source": "wasm",
            "version": "1.2.3",
            "definition": {
                "kind": "n.wasm.test",
                "title": "WASM Test",
                "description": "Execute a test WASM module.",
                "config_schema": {},
                "input_pins": ["in"],
                "output_pins": ["out"]
            },
            "wasm_runtime": {
                "module": "dist/module.wasm",
                "abi": "zebflow-wasm-json-v1"
            }
        }))
        .expect("manifest")
    }

    #[test]
    fn accepts_complete_wasm_definition() {
        validate_node_definition_spec(&wasm_manifest()).expect("valid manifest");
    }

    #[test]
    fn golden_v1_roundtrips_without_schema_drift() {
        let document = decode_node_definition(V1_COMPLETE).expect("decode golden definition");
        let encoded = encode_node_definition(document.metadata, document.spec)
            .expect("encode golden definition");
        assert_eq!(encoded, V1_COMPLETE);
    }

    #[test]
    fn rejects_definition_identity_mismatch_and_release_version() {
        let document = decode_node_definition(V1_COMPLETE).expect("definition");
        let mut wrong_name = document.metadata.clone();
        wrong_name.name = "n.example.other".into();
        assert!(encode_node_definition(wrong_name, document.spec.clone()).is_err());

        let mut versioned = document.metadata;
        versioned.version = Some("1.0.0".into());
        assert!(encode_node_definition(versioned, document.spec).is_err());
    }

    #[test]
    fn rejects_unknown_fields_and_oversized_documents() {
        let mut value: serde_json::Value = serde_json::from_slice(V1_COMPLETE).expect("json");
        value["spec"]["unknown"] = serde_json::json!(true);
        assert!(decode_node_definition(&serde_json::to_vec(&value).expect("json bytes")).is_err());
        assert!(decode_node_definition(&vec![b' '; MAX_NODE_DEFINITION_BYTES + 1]).is_err());
    }

    #[test]
    fn bundle_normalization_preserves_payload_schemas() {
        let package: MultiNodePackageDefinition = serde_json::from_value(serde_json::json!({
            "package": "schema-test",
            "version": "1.0.0",
            "title": "Schema Test",
            "description": "Prove that package normalization preserves node payload schemas.",
            "functions": {"main": "functions/main.zf.json"},
            "nodes": [{
                "kind": "n.c.schema.test",
                "title": "Schema Test",
                "description": "Return a typed result.",
                "main": "main",
                "definition": {
                    "config_schema": {},
                    "input_schema": {"type": "object", "required": ["value"]},
                    "output_schema": {"type": "object", "required": ["result"]},
                    "examples": [{
                        "title": "Typed result",
                        "input": {"value": 2},
                        "output": {"result": 4}
                    }],
                    "failure_semantics": [{
                        "code": "FW_SCHEMA_TEST_INVALID",
                        "description": "The value is invalid.",
                        "retryable": false
                    }],
                    "input_pins": ["in"],
                    "output_pins": ["out"]
                }
            }]
        }))
        .expect("bundle");
        let normalized = normalize_node_bundle(&package).expect("normalize");
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0].definition.input_schema["required"][0],
            "value"
        );
        assert_eq!(
            normalized[0].definition.output_schema["required"][0],
            "result"
        );
        assert_eq!(normalized[0].definition.examples[0].title, "Typed result");
        assert_eq!(
            normalized[0].definition.failure_semantics[0].code,
            "FW_SCHEMA_TEST_INVALID"
        );
        let api_item = crate::pipeline::NodeContractItem::from(normalized[0].definition.clone());
        assert_eq!(api_item.examples, normalized[0].definition.examples);
        assert_eq!(
            api_item.failure_semantics,
            normalized[0].definition.failure_semantics
        );
    }

    #[test]
    fn rejects_invalid_examples_and_duplicate_failure_codes() {
        let mut manifest = wasm_manifest();
        manifest
            .definition
            .examples
            .push(crate::pipeline::NodeExample {
                title: String::new(),
                config: serde_json::json!([]),
                ..Default::default()
            });
        assert!(validate_node_definition_spec(&manifest).is_err());

        manifest.definition.examples.clear();
        manifest.definition.failure_semantics = vec![
            crate::pipeline::NodeFailureSemantic {
                code: "FW_DUPLICATE".into(),
                description: "First meaning.".into(),
                ..Default::default()
            },
            crate::pipeline::NodeFailureSemantic {
                code: "FW_DUPLICATE".into(),
                description: "Second meaning.".into(),
                ..Default::default()
            },
        ];
        assert!(validate_node_definition_spec(&manifest).is_err());
    }

    #[test]
    fn rejects_wasm_definition_without_runtime() {
        let mut manifest = wasm_manifest();
        manifest.wasm_runtime = None;
        assert!(validate_node_definition_spec(&manifest).is_err());
    }

    #[test]
    fn rejects_unsafe_wasm_module_path() {
        let mut manifest = wasm_manifest();
        manifest.wasm_runtime.as_mut().expect("runtime").module = "../module.wasm".into();
        assert!(validate_node_definition_spec(&manifest).is_err());
    }

    #[test]
    fn rejects_mixed_wasm_and_composite_runtime() {
        let mut manifest = wasm_manifest();
        manifest.main_function = Some("main".into());
        assert!(validate_node_definition_spec(&manifest).is_err());
    }

    #[test]
    fn rejects_duplicate_pins() {
        let mut manifest = wasm_manifest();
        manifest.definition.output_pins = vec!["out".into(), "out".into()];
        assert!(validate_node_definition_spec(&manifest).is_err());
    }
}
