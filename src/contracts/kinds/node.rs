use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use std::collections::HashSet;
use std::path::{Component, Path};

use crate::platform::model::{MultiNodePackageDefinition, NodePackageManifest, NodePackageSource};

/// Canonical source package containing one or more installable nodes.
pub struct NodeBundleContract;

impl PlatformContract for NodeBundleContract {
    type Spec = MultiNodePackageDefinition;
    const KIND: ContractKind = ContractKind::NodeBundle;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        validate_release_metadata(metadata, &spec.package, &spec.version)?;
        if spec.nodes.is_empty() {
            return Err(ContractError::invalid(
                "NodeBundle must contain at least one node",
            ));
        }
        Ok(())
    }
}

/// Canonical installed manifest for one exploded node definition.
pub struct NodeDefinitionContract;

impl PlatformContract for NodeDefinitionContract {
    type Spec = NodePackageManifest;
    const KIND: ContractKind = ContractKind::NodeDefinition;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        validate_release_metadata(metadata, &spec.definition.kind, &spec.version)?;
        validate_node_definition_spec(spec)
    }
}

/// Validate all context-free rules of one durable installed node definition.
///
/// Project-specific collision checks remain the node registry's responsibility.
pub fn validate_node_definition_spec(spec: &NodePackageManifest) -> Result<(), ContractError> {
    validate_release_version(&spec.version)?;
    crate::pipeline::nodes::validate_node_definition_contract(&spec.definition).map_err(
        |errors| {
            ContractError::invalid(format!(
                "node definition is incomplete: {}",
                errors.join("; ")
            ))
        },
    )?;

    validate_unique_tokens("spec.definition.input_pins", &spec.definition.input_pins)?;
    validate_unique_tokens("spec.definition.output_pins", &spec.definition.output_pins)?;
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
    if spec.runtime.is_none() && spec.main_function.is_none() && spec.trigger.is_none() {
        return Err(ContractError::invalid(
            "composite node must declare runtime, main_function, or trigger",
        ));
    }
    if let Some(runtime) = &spec.runtime {
        validate_relative_file("spec.runtime.pipeline", &runtime.pipeline)?;
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
    if spec.runtime.is_some()
        || !spec.functions.is_empty()
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
    if runtime.abi.trim().is_empty() {
        return Err(ContractError::invalid(
            "spec.wasm_runtime.abi must not be empty",
        ));
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
        manifest.runtime = Some(crate::platform::model::CompositeNodeRuntime {
            pipeline: "pipeline.zf.json".into(),
        });
        assert!(validate_node_definition_spec(&manifest).is_err());
    }

    #[test]
    fn rejects_duplicate_pins() {
        let mut manifest = wasm_manifest();
        manifest.definition.output_pins = vec!["out".into(), "out".into()];
        assert!(validate_node_definition_spec(&manifest).is_err());
    }
}
