//! Execution host for nodes provided by installed bundles.
//!
//! A bundle-provided node is dispatched from its package manifest, never from
//! its kind, so the same node may be composite or WASM without the graph
//! knowing. Composite behaviour runs an inner function pipeline; WASM behaviour
//! runs a declared export through [`super::wasm_host`].
//!
//! Package discovery and validation live in
//! `src/platform/services/node_registry.rs`.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::language::DenoSandboxEngine;
use crate::pipeline::PipelineContext;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineError;

use super::basic::BasicPipelineEngine;

pub(super) async fn execute_installed_node(
    kind: String,
    config: Value,
    platform: Arc<crate::platform::services::PlatformService>,
    input: crate::pipeline::nodes::NodeExecutionInput,
) -> Result<Vec<crate::pipeline::nodes::NodeExecutionOutput>, PipelineError> {
    use crate::platform::model::NodePackageSource;

    let owner = input
        .metadata
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let project = input
        .metadata
        .get("project")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let Some(manifest) = platform.node_registry.get_manifest(&owner, &project, &kind) else {
        return Err(PipelineError::new(
            "FW_NODE_PACKAGE_NOT_FOUND",
            format!("node package for '{kind}' is not installed"),
        ));
    };

    if manifest.trigger.is_some() {
        return execute_installed_trigger(&kind, &config, &platform, vec![input]).await;
    }

    match manifest.source {
        NodePackageSource::Wasm => {
            super::wasm_host::execute_wasm_node(kind, config, platform, input).await
        }
        NodePackageSource::Composite => {
            execute_composite_node(&kind, &config, &platform, vec![input]).await
        }
        NodePackageSource::Declarative => Err(PipelineError::new(
            "FW_NODE_PACKAGE_NOT_EXECUTABLE",
            format!("node '{kind}' declares no run binding and is not a trigger"),
        )),
    }
}

/// Runs a WASM trigger handler for one inbound event.
///
/// A failing handler passes the raw payload through, matching the composite
/// handler's behavior so a trigger never drops an event.
fn execute_wasm_trigger_handler(
    kind: &str,
    config: &Value,
    owner: &str,
    project: &str,
    platform: &Arc<crate::platform::services::PlatformService>,
    input: &crate::pipeline::nodes::NodeExecutionInput,
) -> crate::pipeline::nodes::NodeExecutionOutput {
    use crate::pipeline::nodes::NodeExecutionOutput;

    let passthrough = |reason: String| NodeExecutionOutput {
        output_pins: vec!["out".to_string()],
        payload: input.payload.clone(),
        trace: vec![reason],
    };

    let Some(installed) = platform.node_registry.get_by_kind(owner, project, kind) else {
        return passthrough(format!(
            "wasm trigger '{kind}' package not installed, raw passthrough"
        ));
    };
    let Some((module_spec, export)) = installed.manifest.wasm_target() else {
        return passthrough(format!(
            "wasm trigger '{kind}' has no resolvable export, raw passthrough"
        ));
    };
    let handler_input = input
        .payload
        .get("body")
        .cloned()
        .unwrap_or_else(|| input.payload.clone());

    match crate::pipeline::engines::wasm_host::run_wasm_export(
        kind,
        &installed.package_dir,
        module_spec,
        export,
        config,
        &handler_input,
        &input.metadata,
    ) {
        Ok(payload) => NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload,
            trace: vec![format!("wasm trigger '{kind}' export '{export}' ok")],
        },
        Err(err) => {
            eprintln!(
                "wasm_trigger: export '{export}' failed for '{kind}': {}, passing through raw",
                err.message
            );
            passthrough(format!(
                "wasm trigger '{kind}' export error: {}, raw passthrough",
                err.message
            ))
        }
    }
}

async fn execute_installed_trigger(
    kind: &str,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    inputs: Vec<crate::pipeline::nodes::NodeExecutionInput>,
) -> Result<Vec<crate::pipeline::nodes::NodeExecutionOutput>, PipelineError> {
    use crate::pipeline::nodes::NodeExecutionOutput;

    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // The inbound handler is the node's run binding, so a trigger may be
        // implemented as a composite function or as a WASM export.
        let run_binding = platform
            .node_registry
            .get_manifest(&owner, &project, kind)
            .and_then(|manifest| manifest.run.clone());

        if let Some(binding) = run_binding.as_ref()
            && binding.module.is_some()
        {
            results.push(execute_wasm_trigger_handler(
                kind, config, &owner, &project, platform, &input,
            ));
            continue;
        }

        let on_message_fn = run_binding.and_then(|binding| binding.function);

        if let Some(fn_name) = on_message_fn {
            // Load and execute the on_message transform pipeline.
            let graph = match platform.node_registry.load_composite_function(
                &owner,
                &project,
                kind,
                Some(&fn_name),
            ) {
                Ok(g) => g,
                Err(e) => {
                    // If transform function fails to load, pass through raw payload.
                    eprintln!(
                        "composite_trigger: on_message '{}' load failed for '{}': {}, passing through raw",
                        fn_name, kind, e.message
                    );
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: input.payload.clone(),
                        trace: vec![format!(
                            "composite trigger '{}' on_message load error, raw passthrough",
                            kind
                        )],
                    });
                    continue;
                }
            };

            let placeholder_map =
                build_composite_placeholder_map(kind, config, &owner, &project, platform);

            // The input to the transform is the webhook body.
            let transform_input = if let Some(body) = input.payload.get("body") {
                body.clone()
            } else {
                input.payload.clone()
            };

            let ctx = PipelineContext {
                owner: owner.clone(),
                project: project.clone(),
                pipeline: format!("composite_trigger::{}::on_message", kind),
                request_id: format!(
                    "ct-{}-{}",
                    kind,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                route: Default::default(),
                input: transform_input,
                trigger: None,
                placeholder: if placeholder_map.is_empty() {
                    None
                } else {
                    Some(json!(placeholder_map))
                },
            };

            let engine = BasicPipelineEngine::new(
                Arc::new(DenoSandboxEngine::default()),
                crate::rwe::resolve_engine_or_default(None),
                Some(platform.credentials.clone()),
            )
            .with_platform(platform.clone())
            .with_ws_hub(platform.ws_hub.clone())
            .with_state_bus(platform.state_bus.clone())
            .with_data_root(platform.config.data_root.clone());

            match engine.execute_async(&graph, &ctx).await {
                Ok(output) => {
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: output.value,
                        trace: vec![format!("composite trigger '{}' on_message ok", kind)],
                    });
                }
                Err(e) => {
                    // Transform failed — pass through raw payload so pipeline can still work.
                    eprintln!(
                        "composite_trigger: on_message '{}' exec failed for '{}': {}, passing through raw",
                        fn_name, kind, e.message
                    );
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: input.payload.clone(),
                        trace: vec![format!(
                            "composite trigger '{}' on_message error: {}, raw passthrough",
                            kind, e.message
                        )],
                    });
                }
            }
        } else {
            // No on_message — pass through like n.trigger.webhook.
            results.push(NodeExecutionOutput {
                output_pins: vec!["out".to_string()],
                payload: input.payload.clone(),
                trace: vec![format!("composite trigger '{}' passthrough", kind)],
            });
        }
    }

    Ok(results)
}

/// Builds a `$placeholder` map from the composite manifest's credential declarations,
/// resolving each credential's secret fields into named placeholders. This map is
/// injected into the inner pipeline's execution context so `{{ $placeholder.X }}`
/// expressions resolve to actual credential values.
async fn execute_composite_node(
    kind: &str,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    inputs: Vec<crate::pipeline::nodes::NodeExecutionInput>,
) -> Result<Vec<crate::pipeline::nodes::NodeExecutionOutput>, PipelineError> {
    use crate::pipeline::nodes::NodeExecutionOutput;

    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Load the inner pipeline graph from the installed package.
        let graph = match platform
            .node_registry
            .load_composite_pipeline(&owner, &project, kind)
        {
            Ok(g) => g,
            Err(e) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["error".to_string()],
                    payload: json!({"error": format!("{}: {}", e.code, e.message)}),
                    trace: vec![format!("composite '{}' load error: {}", kind, e.message)],
                });
                continue;
            }
        };

        // Build $placeholder map from credential declarations in the manifest.
        let mut placeholder_map =
            build_composite_placeholder_map(kind, config, &owner, &project, platform);

        // Inject non-credential config fields as CONFIG_<KEY> placeholders so
        // inner pipelines can access node config (e.g. model override).
        if let Some(obj) = config.as_object() {
            for (key, val) in obj {
                // Skip credential ID fields — those are already resolved.
                if key.ends_with("_credential_id") || key == "credential_id" {
                    continue;
                }
                let placeholder_key = format!("CONFIG_{}", key.to_uppercase());
                if !placeholder_map.contains_key(&placeholder_key) {
                    placeholder_map.insert(placeholder_key, val.clone());
                }
            }
        }

        // Allow runtime input `__config` to override CONFIG_<KEY> placeholders,
        // enabling dynamic per-run config (e.g. model selection like n.ai.agent).
        if let Some(runtime_cfg) = input.payload.get("__config").and_then(|v| v.as_object()) {
            for (key, val) in runtime_cfg {
                if key.ends_with("_credential_id") || key == "credential_id" {
                    continue;
                }
                let placeholder_key = format!("CONFIG_{}", key.to_uppercase());
                placeholder_map.insert(placeholder_key, val.clone());
            }
        }

        // Build execution context — same pattern as execute_function_pipeline.
        let ctx = PipelineContext {
            owner: owner.clone(),
            project: project.clone(),
            pipeline: format!("composite::{}", kind),
            request_id: format!(
                "composite-{}-{}",
                kind,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input: input.payload.clone(),
            trigger: None,
            placeholder: if placeholder_map.is_empty() {
                None
            } else {
                Some(json!(placeholder_map))
            },
        };

        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            crate::rwe::resolve_engine_or_default(None),
            Some(platform.credentials.clone()),
        )
        .with_platform(platform.clone())
        .with_ws_hub(platform.ws_hub.clone())
        .with_state_bus(platform.state_bus.clone())
        .with_data_root(platform.config.data_root.clone());

        match engine.execute_async(&graph, &ctx).await {
            Ok(output) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["out".to_string()],
                    payload: output.value,
                    trace: vec![format!("composite '{}' ok", kind)],
                });
            }
            Err(e) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["error".to_string()],
                    payload: json!({"error": format!("{}: {}", e.code, e.message)}),
                    trace: vec![format!(
                        "composite '{}' error: {} — {}",
                        kind, e.code, e.message
                    )],
                });
            }
        }
    }

    Ok(results)
}

/// Builds a placeholder name → resolved value map from a composite node's
/// credential declarations and the user's config.
///
/// For each credential declaration in the manifest:
/// 1. Read `config_key` to find which config field holds the credential ID.
/// 2. Fetch the credential from `CredentialService`.
/// 3. For each `placeholders` entry (e.g. `"BOT_TOKEN" -> "bot_token"`),
///    read the secret field and map the placeholder name to the actual value.
pub fn build_composite_placeholder_map(
    kind: &str,
    config: &Value,
    owner: &str,
    project: &str,
    platform: &Arc<crate::platform::services::PlatformService>,
) -> serde_json::Map<String, Value> {
    let mut placeholder_map = serde_json::Map::new();

    let manifest = match platform.node_registry.get_manifest(owner, project, kind) {
        Some(m) => m,
        None => return placeholder_map,
    };

    for cred_decl in &manifest.credentials {
        if cred_decl.config_key.is_empty() || cred_decl.placeholders.is_empty() {
            continue;
        }

        // Read the credential ID from the composite node's config.
        let credential_id = match config.get(&cred_decl.config_key).and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => {
                eprintln!(
                    "composite '{}': config key '{}' is empty or missing",
                    kind, cred_decl.config_key
                );
                continue;
            }
        };

        // Fetch the credential from the credential service.
        let credential =
            match platform
                .credentials
                .get_project_credential(owner, project, &credential_id)
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    eprintln!(
                        "composite '{}': credential '{}' not found",
                        kind, credential_id
                    );
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "composite '{}': failed to fetch credential '{}': {}",
                        kind, credential_id, e.message
                    );
                    continue;
                }
            };

        // Build placeholder entries from the manifest's placeholder map.
        for (placeholder_name, secret_field) in &cred_decl.placeholders {
            if let Some(value) = credential.secret.get(secret_field) {
                placeholder_map.insert(placeholder_name.clone(), value.clone());
            }
        }
    }

    placeholder_map
}
