use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use std::collections::HashSet;
use std::path::{Component, Path};

use crate::pipeline::NodeDefinition;
use crate::platform::model::{
    MultiNodePackageDefinition, NodePackageManifest, NodePackageSource, RunBinding,
};

/// Maximum serialized size of one standalone normalized node definition.
pub const MAX_NODE_DEFINITION_BYTES: usize = 512 * 1024;

/// Maximum serialized size of one `definition.json` bundle document.
pub const MAX_NODE_BUNDLE_BYTES: usize = 4 * 1024 * 1024;

/// Maximum node entries in one bundle.
///
/// Matches `MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS` so a valid bundle can always
/// be recorded in `zeb.lock`.
pub const MAX_NODE_BUNDLE_NODES: usize = 256;

/// Maximum function pipelines declared by one bundle.
pub const MAX_NODE_BUNDLE_FUNCTIONS: usize = 256;

/// Maximum WASM modules declared by one bundle.
pub const MAX_NODE_BUNDLE_MODULES: usize = 64;

/// Maximum credential definitions declared by one bundle.
pub const MAX_NODE_BUNDLE_CREDENTIALS: usize = 32;

/// Maximum regular files inside one installed package directory.
pub const MAX_NODE_BUNDLE_FILES: usize = 4096;

/// WASM ABI supported by the stable v1 node contract.
pub const WASM_JSON_ABI_V1: &str = "zebflow-wasm-json-v1";

/// Reserved kind prefix for every installed node.
///
/// Native nodes may not use it, and an installed node may not use anything else.
pub const INSTALLED_NODE_KIND_PREFIX: &str = "n.x.";

/// Trigger roles a bundle may declare.
pub const BUNDLE_TRIGGER_TYPES: &[&str] = &["webhook", "ws", "ws_client", "cron"];

/// Converts a package slug into its node-kind segment.
///
/// Kind segments allow underscores but not hyphens, so `openai-embedding`
/// owns `n.x.openai_embedding.`.
pub fn package_kind_token(package: &str) -> String {
    package.replace('-', "_")
}

/// Returns the kind prefix a package owns.
pub fn package_kind_namespace(package: &str) -> String {
    format!(
        "{INSTALLED_NODE_KIND_PREFIX}{}.",
        package_kind_token(package)
    )
}

/// Decode one bounded canonical NodeBundle document.
pub fn decode_node_bundle(
    bytes: &[u8],
) -> Result<crate::contracts::ContractDocument<MultiNodePackageDefinition>, ContractError> {
    if bytes.len() > MAX_NODE_BUNDLE_BYTES {
        return Err(ContractError::invalid(format!(
            "NodeBundle exceeds the {MAX_NODE_BUNDLE_BYTES} byte limit"
        )));
    }
    crate::contracts::decode_contract::<NodeBundleContract>(bytes)
}

/// Encode one bounded canonical NodeBundle document.
pub fn encode_node_bundle(
    metadata: ContractMetadata,
    spec: MultiNodePackageDefinition,
) -> Result<Vec<u8>, ContractError> {
    let bytes = crate::contracts::encode_contract::<NodeBundleContract>(metadata, spec)?;
    if bytes.len() > MAX_NODE_BUNDLE_BYTES {
        return Err(ContractError::invalid(format!(
            "NodeBundle exceeds the {MAX_NODE_BUNDLE_BYTES} byte limit"
        )));
    }
    Ok(bytes)
}

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
        validate_release_version(&spec.version)?;
        if spec.title.trim().is_empty() || spec.description.trim().is_empty() {
            return Err(ContractError::invalid(
                "spec.title and spec.description must not be empty",
            ));
        }
        if !spec.icon.is_empty() {
            validate_relative_file("spec.icon", &spec.icon)?;
        }

        validate_limit("spec.nodes", spec.nodes.len(), MAX_NODE_BUNDLE_NODES)?;
        validate_limit(
            "spec.functions",
            spec.functions.len(),
            MAX_NODE_BUNDLE_FUNCTIONS,
        )?;
        validate_limit("spec.modules", spec.modules.len(), MAX_NODE_BUNDLE_MODULES)?;
        validate_limit(
            "spec.credentials",
            spec.credentials.len(),
            MAX_NODE_BUNDLE_CREDENTIALS,
        )?;

        for (name, path) in &spec.functions {
            validate_artifact_name(&format!("spec.functions.{name}"), name)?;
            validate_relative_file(&format!("spec.functions.{name}"), path)?;
        }
        for (name, module) in &spec.modules {
            validate_artifact_name(&format!("spec.modules.{name}"), name)?;
            validate_relative_file(&format!("spec.modules.{name}.path"), &module.path)?;
            if module.abi != WASM_JSON_ABI_V1 {
                return Err(ContractError::invalid(format!(
                    "spec.modules.{name}.abi must be '{WASM_JSON_ABI_V1}'"
                )));
            }
        }

        let mut credential_kinds = HashSet::new();
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
            if !credential_kinds.insert(credential.kind.as_str()) {
                return Err(ContractError::invalid(format!(
                    "spec.credentials declares kind '{}' more than once",
                    credential.kind
                )));
            }
        }

        if spec.nodes.is_empty() {
            return Err(ContractError::invalid(
                "NodeBundle must contain at least one node",
            ));
        }

        let namespace = package_kind_namespace(&spec.package);
        let manifests = normalize_node_bundle(spec)?;
        let mut kinds = HashSet::new();
        let mut used_functions = HashSet::new();
        let mut used_modules = HashSet::new();

        for (entry, manifest) in spec.nodes.iter().zip(&manifests) {
            let kind = &manifest.definition.kind;
            if !kind.starts_with(&namespace) || kind.len() == namespace.len() {
                return Err(ContractError::invalid(format!(
                    "node kind '{kind}' must start with '{namespace}' owned by package '{}'",
                    spec.package
                )));
            }
            if !kinds.insert(kind.clone()) {
                return Err(ContractError::invalid(format!(
                    "spec.nodes contains duplicate node kind '{kind}'"
                )));
            }
            if !entry.icon.is_empty() {
                validate_relative_file(&format!("spec.nodes[{kind}].icon"), &entry.icon)?;
            }
            for name in entry.uses_credentials.iter() {
                if !credential_kinds.contains(name.as_str()) {
                    return Err(ContractError::invalid(format!(
                        "node '{kind}' uses undeclared credential kind '{name}'"
                    )));
                }
            }
            validate_node_definition_spec(manifest)?;
            collect_run_references(manifest, &mut used_functions, &mut used_modules);
        }

        for name in spec.functions.keys() {
            if !used_functions.contains(name.as_str()) {
                return Err(ContractError::invalid(format!(
                    "spec.functions.{name} is declared but no node references it"
                )));
            }
        }
        for name in spec.modules.keys() {
            if !used_modules.contains(name.as_str()) {
                return Err(ContractError::invalid(format!(
                    "spec.modules.{name} is declared but no node references it"
                )));
            }
        }
        Ok(())
    }
}

fn validate_limit(path: &str, actual: usize, limit: usize) -> Result<(), ContractError> {
    if actual > limit {
        return Err(ContractError::invalid(format!(
            "{path} exceeds the limit of {limit}"
        )));
    }
    Ok(())
}

fn validate_artifact_name(path: &str, value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ContractError::invalid(format!(
            "{path} must be a non-empty name of letters, digits, '-', '_', or '.'"
        )));
    }
    Ok(())
}

fn collect_run_references<'a>(
    manifest: &'a NodePackageManifest,
    functions: &mut HashSet<&'a str>,
    modules: &mut HashSet<&'a str>,
) {
    let mut record = |binding: &'a RunBinding| {
        if let Some(name) = binding.function.as_deref() {
            functions.insert(name);
        }
        if let Some(name) = binding.module.as_deref() {
            modules.insert(name);
        }
    };
    if let Some(run) = manifest.run.as_ref() {
        record(run);
    }
    if let Some(lifecycle) = manifest.lifecycle.as_ref() {
        if let Some(hook) = lifecycle.on_activate.as_ref() {
            record(hook);
        }
        if let Some(hook) = lifecycle.on_deactivate.as_ref() {
            record(hook);
        }
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
        // The implementation type always follows the run binding. A malformed
        // binding is reported by `validate_node_definition_spec`, so the
        // placeholder here never reaches a validated manifest.
        let source = node
            .run
            .as_ref()
            .and_then(RunBinding::source)
            .unwrap_or(NodePackageSource::Declarative);
        manifests.push(NodePackageManifest {
            source,
            version: package.version.clone(),
            definition,
            credentials: package
                .credentials
                .iter()
                .filter(|credential| node.uses_credentials.contains(&credential.kind))
                .cloned()
                .collect(),
            run: node.run.clone(),
            trigger: node.trigger.clone(),
            lifecycle: node.lifecycle.clone(),
            functions: package.functions.clone(),
            modules: package.modules.clone(),
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
    validate_installed_kind(&spec.definition.kind)?;
    validate_credentials(spec)?;
    validate_role_and_run(spec)
}

/// Every installed node lives under the reserved `n.x.` namespace.
fn validate_installed_kind(kind: &str) -> Result<(), ContractError> {
    if !kind.starts_with(INSTALLED_NODE_KIND_PREFIX) {
        return Err(ContractError::invalid(format!(
            "installed node kind '{kind}' must start with '{INSTALLED_NODE_KIND_PREFIX}'"
        )));
    }
    Ok(())
}

/// Validates the node's role, its run binding, and every artifact reference.
fn validate_role_and_run(spec: &NodePackageManifest) -> Result<(), ContractError> {
    let kind = &spec.definition.kind;

    if let Some(trigger) = &spec.trigger {
        if !BUNDLE_TRIGGER_TYPES.contains(&trigger.trigger_type.as_str()) {
            return Err(ContractError::invalid(format!(
                "node '{kind}' declares unsupported trigger type '{}'; expected one of {}",
                trigger.trigger_type,
                BUNDLE_TRIGGER_TYPES.join(", ")
            )));
        }
        if let Some(template) = &trigger.path_template {
            validate_path_template(kind, template, &spec.definition.config_schema)?;
        }
        if !spec.definition.input_pins.is_empty() {
            return Err(ContractError::invalid(format!(
                "trigger node '{kind}' must not declare input pins"
            )));
        }
    } else if spec.lifecycle.is_some() {
        return Err(ContractError::invalid(format!(
            "node '{kind}' declares lifecycle hooks without a trigger"
        )));
    }

    match &spec.run {
        None => {
            if spec.trigger.is_none() {
                return Err(ContractError::invalid(format!(
                    "action node '{kind}' must declare a run binding"
                )));
            }
            if spec.source != NodePackageSource::Declarative {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' has no run binding but is not declarative"
                )));
            }
        }
        Some(run) => {
            validate_run_binding(kind, "run", run, spec)?;
            let derived = run.source().expect("validated binding resolves a source");
            if spec.source != derived {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' source must be derived from its run binding"
                )));
            }
        }
    }

    if let Some(lifecycle) = &spec.lifecycle {
        if let Some(hook) = &lifecycle.on_activate {
            validate_run_binding(kind, "lifecycle.on_activate", hook, spec)?;
        }
        if let Some(hook) = &lifecycle.on_deactivate {
            validate_run_binding(kind, "lifecycle.on_deactivate", hook, spec)?;
        }
    }
    Ok(())
}

/// Validates one run binding form and resolves it against declared artifacts.
fn validate_run_binding(
    kind: &str,
    path: &str,
    binding: &RunBinding,
    spec: &NodePackageManifest,
) -> Result<(), ContractError> {
    match binding.source() {
        Some(NodePackageSource::Composite) => {
            let name = binding.function.as_deref().unwrap_or_default();
            if !spec.functions.contains_key(name) {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' {path} references undeclared function '{name}'"
                )));
            }
        }
        Some(NodePackageSource::Wasm) => {
            let module = binding.module.as_deref().unwrap_or_default();
            let export = binding.export.as_deref().unwrap_or_default();
            let Some(spec_module) = spec.modules.get(module) else {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' {path} references undeclared module '{module}'"
                )));
            };
            if spec_module.abi != WASM_JSON_ABI_V1 {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' {path} module '{module}' must use ABI '{WASM_JSON_ABI_V1}'"
                )));
            }
            validate_relative_file(&format!("{path}.module"), &spec_module.path)?;
            if export.trim().is_empty() {
                return Err(ContractError::invalid(format!(
                    "node '{kind}' {path} must name a WASM export"
                )));
            }
        }
        Some(NodePackageSource::Declarative) | None => {
            return Err(ContractError::invalid(format!(
                "node '{kind}' {path} must declare either 'function' or both 'module' and 'export'"
            )));
        }
    }
    Ok(())
}

/// Every `{{ key }}` in a trigger path template must be a declared config key.
fn validate_path_template(
    kind: &str,
    template: &str,
    config_schema: &serde_json::Value,
) -> Result<(), ContractError> {
    let properties = config_schema
        .get("properties")
        .and_then(serde_json::Value::as_object);
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err(ContractError::invalid(format!(
                "node '{kind}' trigger path_template has an unterminated placeholder"
            )));
        };
        let key = after[..end].trim();
        if key.is_empty() {
            return Err(ContractError::invalid(format!(
                "node '{kind}' trigger path_template has an empty placeholder"
            )));
        }
        if !properties.is_some_and(|values| values.contains_key(key)) {
            return Err(ContractError::invalid(format!(
                "node '{kind}' trigger path_template references undeclared config key '{key}'"
            )));
        }
        rest = &after[end + 2..];
    }
    Ok(())
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
    use crate::contracts::ContractDocument;

    const V1_DEFINITION: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-definition/v1-complete.json");
    const V1_COMPOSITE: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-bundle/v1-composite.json");
    const V1_WASM: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-bundle/v1-wasm.json");
    const V1_MIXED: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/node-bundle/v1-mixed.json");

    fn bundle(bytes: &[u8]) -> ContractDocument<MultiNodePackageDefinition> {
        decode_node_bundle(bytes).expect("decode bundle")
    }

    fn mutate(bytes: &[u8], mutation: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).expect("json");
        mutation(&mut value);
        serde_json::to_vec(&value).expect("json bytes")
    }

    // ── Golden round-trips ──────────────────────────────────────────────

    #[test]
    fn golden_bundles_roundtrip_without_schema_drift() {
        for bytes in [V1_COMPOSITE, V1_WASM, V1_MIXED] {
            let document = bundle(bytes);
            let encoded =
                encode_node_bundle(document.metadata, document.spec).expect("encode bundle");
            assert_eq!(encoded, bytes);
        }
    }

    #[test]
    fn golden_definition_roundtrips_without_schema_drift() {
        let document = decode_node_definition(V1_DEFINITION).expect("decode golden definition");
        let encoded =
            encode_node_definition(document.metadata, document.spec).expect("encode definition");
        assert_eq!(encoded, V1_DEFINITION);
    }

    // ── Implementation is derived, never authored ───────────────────────

    #[test]
    fn implementation_type_follows_the_run_binding() {
        let manifests = normalize_node_bundle(&bundle(V1_MIXED).spec).expect("normalize");
        let sources: Vec<_> = manifests
            .iter()
            .map(|manifest| (manifest.definition.kind.as_str(), manifest.source))
            .collect();
        assert_eq!(
            sources,
            vec![
                ("n.x.mixed.load", NodePackageSource::Composite),
                ("n.x.mixed.crunch", NodePackageSource::Wasm),
                ("n.x.mixed.inbox", NodePackageSource::Declarative),
            ]
        );
    }

    #[test]
    fn two_wasm_nodes_resolve_distinct_exports() {
        let manifests = normalize_node_bundle(&bundle(V1_WASM).spec).expect("normalize");
        let targets: Vec<_> = manifests
            .iter()
            .map(|manifest| {
                let (module, export) = manifest.wasm_target().expect("wasm target");
                (module.path.as_str(), export)
            })
            .collect();
        assert_eq!(
            targets,
            vec![
                ("wasm/core.wasm", "gb_train"),
                ("wasm/core.wasm", "gb_score"),
            ]
        );
    }

    #[test]
    fn credentials_are_scoped_to_the_nodes_that_use_them() {
        let manifests = normalize_node_bundle(&bundle(V1_MIXED).spec).expect("normalize");
        let load = &manifests[0];
        let crunch = &manifests[1];
        assert_eq!(load.definition.kind, "n.x.mixed.load");
        assert_eq!(
            load.credentials
                .iter()
                .map(|credential| credential.kind.as_str())
                .collect::<Vec<_>>(),
            vec!["mixed_api"]
        );
        // A node that uses no credential carries no credential obligation,
        // which is what allows one bundle to provide unrelated nodes.
        assert!(crunch.credentials.is_empty());
    }

    // ── Identity and release rules ──────────────────────────────────────

    #[test]
    fn rejects_metadata_and_spec_identity_mismatch() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["metadata"]["name"] = serde_json::json!("other");
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["metadata"]["version"] = serde_json::json!("9.9.9");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_wrong_kind_and_future_version() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["kind"] = serde_json::json!("Pipeline");
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["apiVersion"] = serde_json::json!("zebflow.com/v2");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_invalid_slug_and_release_version() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["metadata"]["name"] = serde_json::json!("Bad_Slug");
                value["spec"]["package"] = serde_json::json!("Bad_Slug");
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["metadata"]["version"] = serde_json::json!("1.0");
                value["spec"]["version"] = serde_json::json!("1.0");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_unknown_fields_and_oversized_documents() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["unknown"] = serde_json::json!(true);
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["source"] = serde_json::json!("composite");
            }))
            .is_err(),
            "an authored source field must not be accepted"
        );
        assert!(decode_node_bundle(&vec![b' '; MAX_NODE_BUNDLE_BYTES + 1]).is_err());
    }

    // ── Kind ownership ──────────────────────────────────────────────────

    #[test]
    fn package_owns_its_kind_namespace() {
        assert_eq!(package_kind_namespace("ml"), "n.x.ml.");
        assert_eq!(
            package_kind_namespace("openai-embedding"),
            "n.x.openai_embedding."
        );
    }

    #[test]
    fn rejects_kind_outside_the_package_namespace() {
        for kind in [
            "n.x.other.load",
            "n.c.composite.load",
            "n.wasm.composite.load",
            "n.x.composite",
        ] {
            assert!(
                decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                    value["spec"]["nodes"][0]["kind"] = serde_json::json!(kind);
                }))
                .is_err(),
                "kind '{kind}' must be rejected"
            );
        }
    }

    #[test]
    fn rejects_duplicate_node_kind() {
        assert!(
            decode_node_bundle(&mutate(V1_WASM, |value| {
                value["spec"]["nodes"][1]["kind"] = serde_json::json!("n.x.wasmpkg.train");
            }))
            .is_err()
        );
    }

    // ── Run binding ─────────────────────────────────────────────────────

    #[test]
    fn rejects_module_without_export() {
        assert!(
            decode_node_bundle(&mutate(V1_WASM, |value| {
                value["spec"]["nodes"][0]["run"]
                    .as_object_mut()
                    .expect("run object")
                    .remove("export");
            }))
            .is_err(),
            "a default export symbol would make two WASM nodes collide"
        );
    }

    #[test]
    fn rejects_mixed_and_empty_run_forms() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["run"] =
                    serde_json::json!({ "function": "load", "module": "core", "export": "run" });
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["run"] = serde_json::json!({});
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_action_node_without_run() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]
                    .as_object_mut()
                    .expect("node object")
                    .remove("run");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_unresolvable_function_and_module_references() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["run"] = serde_json::json!({ "function": "missing" });
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_WASM, |value| {
                value["spec"]["nodes"][0]["run"] =
                    serde_json::json!({ "module": "missing", "export": "gb_train" });
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_declared_but_unreferenced_artifacts() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["functions"]["orphan"] =
                    serde_json::json!("functions/orphan.zf.json");
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_WASM, |value| {
                value["spec"]["modules"]["orphan"] = serde_json::json!({
                    "path": "wasm/orphan.wasm",
                    "abi": WASM_JSON_ABI_V1
                });
            }))
            .is_err()
        );
    }

    // ── Trigger and lifecycle ───────────────────────────────────────────

    #[test]
    fn rejects_lifecycle_on_a_non_trigger_node() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["lifecycle"] =
                    serde_json::json!({ "on_activate": { "function": "load" } });
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_unsupported_trigger_type_and_input_pins() {
        assert!(
            decode_node_bundle(&mutate(V1_MIXED, |value| {
                value["spec"]["nodes"][2]["trigger"]["type"] = serde_json::json!("carrier_pigeon");
            }))
            .is_err()
        );
        assert!(
            decode_node_bundle(&mutate(V1_MIXED, |value| {
                value["spec"]["nodes"][2]["definition"]["input_pins"] = serde_json::json!(["in"]);
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_path_template_with_undeclared_config_key() {
        assert!(
            decode_node_bundle(&mutate(V1_MIXED, |value| {
                value["spec"]["nodes"][2]["trigger"]["path_template"] =
                    serde_json::json!("/mixed/{{ nope }}");
            }))
            .is_err()
        );
    }

    #[test]
    fn accepts_a_wasm_trigger_handler() {
        let bytes = mutate(V1_MIXED, |value| {
            value["spec"]["nodes"][2]["run"] =
                serde_json::json!({ "module": "core", "export": "on_event" });
        });
        let document = decode_node_bundle(&bytes).expect("wasm trigger is expressible");
        let manifests = normalize_node_bundle(&document.spec).expect("normalize");
        assert_eq!(manifests[2].source, NodePackageSource::Wasm);
        assert!(manifests[2].trigger.is_some());
    }

    // ── Artifacts, ABI, credentials ─────────────────────────────────────

    #[test]
    fn rejects_unsafe_artifact_paths() {
        for path in [
            "../escape.zf.json",
            "/abs/escape.zf.json",
            "./escape.zf.json",
        ] {
            assert!(
                decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                    value["spec"]["functions"]["load"] = serde_json::json!(path);
                }))
                .is_err(),
                "path '{path}' must be rejected"
            );
        }
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["icon"] = serde_json::json!("../icon.svg");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_unsupported_wasm_abi() {
        assert!(
            decode_node_bundle(&mutate(V1_WASM, |value| {
                value["spec"]["modules"]["core"]["abi"] = serde_json::json!("zebflow-wasm-json-v2");
            }))
            .is_err()
        );
    }

    #[test]
    fn rejects_credential_problems() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"][0]["uses_credentials"] = serde_json::json!(["nope"]);
            }))
            .is_err(),
            "a node may not use an undeclared credential kind"
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                let credential = value["spec"]["credentials"][0].clone();
                value["spec"]["credentials"] = serde_json::json!([credential.clone(), credential]);
            }))
            .is_err(),
            "duplicate credential kinds must be rejected"
        );
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["credentials"][0]["config_key"] = serde_json::json!("absent_key");
            }))
            .is_err(),
            "a used credential must map to a declared config key"
        );
    }

    #[test]
    fn rejects_limit_violations() {
        assert!(
            decode_node_bundle(&mutate(V1_COMPOSITE, |value| {
                value["spec"]["nodes"] = serde_json::json!([]);
            }))
            .is_err()
        );
    }
}
