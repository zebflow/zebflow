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

use crate::pipeline::PipelineContext;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineError;
use crate::pipeline::security::BundleEgress;

use super::basic::BasicPipelineEngine;

pub(super) async fn execute_installed_node(
    kind: String,
    config: Value,
    platform: Arc<crate::platform::services::PlatformService>,
    parent_egress: Option<Arc<BundleEgress>>,
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

    // The manifest resolved here names the bundle this dispatch is inside, so
    // this is where its declaration becomes the policy for everything below.
    // A bundle declaring no host still gets a policy: the guards that care
    // whether a bundle is running at all must not be escapable by silence.
    let egress = Some(Arc::new(BundleEgress::extend(
        parent_egress.as_deref(),
        &manifest.package,
        &manifest.hosts,
    )));

    if manifest.trigger.is_some() {
        return execute_installed_trigger(&kind, &config, &platform, egress, vec![input]).await;
    }

    match manifest.source {
        NodePackageSource::Wasm => {
            super::wasm_host::execute_wasm_node(kind, config, platform, input).await
        }
        NodePackageSource::Composite => {
            execute_composite_node(&kind, &config, &platform, egress, vec![input]).await
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
    egress: Option<Arc<BundleEgress>>,
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
                Arc::new(platform.project_sandbox(&owner, &project)),
                crate::rwe::resolve_engine_or_default(None),
                Some(platform.credentials.clone()),
            )
            .with_platform(platform.clone())
            .with_ws_hub(platform.ws_hub.clone())
            .with_state_bus(platform.state_bus.clone())
            .with_data_root(platform.config.data_root.clone())
            .with_bundle_egress(egress.clone());

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
    egress: Option<Arc<BundleEgress>>,
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
            Arc::new(platform.project_sandbox(&owner, &project)),
            crate::rwe::resolve_engine_or_default(None),
            Some(platform.credentials.clone()),
        )
        .with_platform(platform.clone())
        .with_ws_hub(platform.ws_hub.clone())
        .with_state_bus(platform.state_bus.clone())
        .with_data_root(platform.config.data_root.clone())
        .with_bundle_egress(egress.clone());

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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use serde_json::{Value, json};

    use crate::contracts::{ContractMetadata, encode_contract};
    use crate::pipeline::nodes::NodeExecutionInput;
    use crate::platform::services::PlatformService;

    const OWNER: &str = "superadmin";
    const PROJECT: &str = "default";

    fn platform_for(root: &Path) -> Arc<PlatformService> {
        Arc::new(
            PlatformService::from_config(crate::platform::model::PlatformConfig {
                data_root: root.to_path_buf(),
                default_password: "test-password".to_string(),
                ..Default::default()
            })
            .expect("platform service"),
        )
    }

    fn http_get(url: &str) -> Value {
        json!({ "method": "GET", "url": url, "response_type": "json" })
    }

    /// Writes a one-node composite bundle whose function runs exactly one inner
    /// node, so what that node reaches is the only variable.
    fn write_bundle(root: &Path, slug: &str, hosts: Value, call_kind: &str, call_config: Value) {
        let package_dir = root
            .join("users/superadmin/default/data/hub/nodes")
            .join(slug);
        std::fs::create_dir_all(package_dir.join("functions")).expect("package dirs");

        let spec: crate::platform::model::MultiNodePackageDefinition =
            serde_json::from_value(json!({
                "package": slug,
                "version": "1.0.0",
                "title": "Host Check",
                "description": "A composite bundle that reaches one external host.",
                "icon": "icon.svg",
                "hosts": hosts,
                "functions": { "main": "functions/main.zf.json" },
                "nodes": [{
                    "kind": format!("n.x.{}.call", slug),
                    "title": "Call",
                    "description": "Run the bundle's outbound request.",
                    "icon": "icon.svg",
                    "run": { "function": "main" },
                    "definition": {
                        "config_schema": {},
                        "input_schema": {"type": "object"},
                        "output_schema": {"type": "object"},
                        "input_pins": ["in"],
                        "output_pins": ["out"]
                    }
                }]
            }))
            .expect("bundle spec");
        let mut metadata = ContractMetadata::named(slug);
        metadata.version = Some("1.0.0".into());
        let bytes = encode_contract::<crate::contracts::kinds::NodeBundleContract>(metadata, spec)
            .expect("bundle contract");
        std::fs::write(package_dir.join("definition.json"), bytes).expect("definition");
        std::fs::write(
            package_dir.join("icon.svg"),
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
        )
        .expect("icon");

        let function: crate::pipeline::PipelineGraph = serde_json::from_value(json!({
            "id": format!("{slug}-main"),
            "entry_nodes": ["trigger"],
            "nodes": [
                {
                    "id": "trigger",
                    "kind": "n.trigger.function",
                    "input_pins": [],
                    "output_pins": ["out"],
                    "config": {}
                },
                {
                    "id": "call",
                    "kind": call_kind,
                    "input_pins": ["in"],
                    "output_pins": ["out"],
                    "config": call_config
                }
            ],
            "edges": [
                { "from_node": "trigger", "from_pin": "out", "to_node": "call", "to_pin": "in" }
            ]
        }))
        .expect("function graph");
        let bytes = encode_contract::<crate::contracts::kinds::PipelineContract>(
            ContractMetadata::named(format!("{slug}-main")),
            function.into(),
        )
        .expect("function contract");
        std::fs::write(package_dir.join("functions/main.zf.json"), bytes).expect("function");
    }

    async fn run_bundle_node(platform: &Arc<PlatformService>, slug: &str) -> Value {
        let outputs = super::execute_installed_node(
            format!("n.x.{slug}.call"),
            json!({}),
            platform.clone(),
            None,
            NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({ "owner": OWNER, "project": PROJECT }),
                bus: None,
            },
        )
        .await
        .expect("the composite node itself dispatches");
        outputs.into_iter().next().expect("one emission").payload
    }

    /// The hole `spec.hosts` was declared to close: a bundle names one host and
    /// composes an HTTP node that could reach any other.
    ///
    /// The refused request is made by an *inner* node of the bundle's function
    /// pipeline, never named by the graph the project wrote, so this is the
    /// evidence that the declaration covers the subtree and not just the node.
    #[tokio::test]
    async fn an_inner_node_may_not_reach_a_host_its_bundle_did_not_declare() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "undeclared",
            json!(["declared.invalid"]),
            "n.http.request",
            http_get("https://elsewhere.invalid/steal"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "undeclared").await;
        assert_eq!(
            payload["error"],
            json!(
                "FW_EGRESS_UNDECLARED_HOST: n.http.request outbound host 'elsewhere.invalid' is \
                 not declared by node bundle 'undeclared' (spec.hosts: declared.invalid)"
            ),
            "the refusal names the host and the bundle whose list refused it"
        );
    }

    /// A declared host is admitted by the bundle guard. The request then fails
    /// on the network, because `.invalid` never resolves — which is the point:
    /// the run got past the declaration and out to DNS.
    #[tokio::test]
    async fn a_declared_host_passes_the_bundle_guard() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "declaredhost",
            json!(["declared.invalid"]),
            "n.http.request",
            http_get("https://declared.invalid/embed"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "declaredhost").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            !error.contains("FW_EGRESS_UNDECLARED_HOST"),
            "the declared host was admitted, got: {error}"
        );
    }

    /// Empty and absent `spec.hosts` are the same bytes, so an empty list
    /// cannot deny without breaking every bundle published before enforcement.
    #[tokio::test]
    async fn a_bundle_that_declares_no_hosts_is_unrestricted() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "nohosts",
            json!([]),
            "n.http.request",
            http_get("https://anywhere.invalid/x"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "nohosts").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            !error.contains("FW_EGRESS_UNDECLARED_HOST"),
            "an empty declaration restricts nothing, got: {error}"
        );
    }

    /// A bundle composing another bundle's node stays answerable for what it
    /// set in motion, so the inner subtree satisfies both declarations.
    #[tokio::test]
    async fn a_nested_bundle_does_not_escape_the_one_that_composed_it() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "outer",
            json!(["api.outer.invalid"]),
            "n.x.inner.call",
            json!({}),
        );
        write_bundle(
            root.path(),
            "inner",
            json!(["api.inner.invalid"]),
            "n.http.request",
            http_get("https://api.inner.invalid/x"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundles register");

        // Run directly, the inner bundle reaches its own declared host.
        let payload = run_bundle_node(&platform, "inner").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            !error.contains("FW_EGRESS_UNDECLARED_HOST"),
            "the inner bundle's own host is fine on its own, got: {error}"
        );

        // Reached through the outer bundle, the same request answers to the
        // outer declaration too, which does not name that host.
        let payload = run_bundle_node(&platform, "outer").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            error.contains("FW_EGRESS_UNDECLARED_HOST")
                && error.contains("'api.inner.invalid'")
                && error.contains("node bundle 'outer'"),
            "the outer bundle's list refuses the nested request, got: {error}"
        );
    }

    /// A network node whose destination never reaches a guard as a URL cannot
    /// be checked, so inside a governed subtree it is refused rather than
    /// quietly becoming the way out of the declaration.
    #[tokio::test]
    async fn a_network_node_that_cannot_be_host_checked_is_refused() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "agentpkg",
            json!(["api.openai.com"]),
            "n.ai.agent",
            json!({}),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "agentpkg").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            error.contains("FW_EGRESS_UNCHECKED_NODE")
                && error.contains("'n.ai.agent'")
                && error.contains("node bundle 'agentpkg'"),
            "the refusal names the node and the bundle, got: {error}"
        );
    }

    /// Declaring nothing must not be the cheap way to reach an unreadable
    /// destination. A bundle with an empty `spec.hosts` is unrestricted in
    /// which hosts it may name, and refused just the same for a node whose
    /// host never reaches the guard.
    #[tokio::test]
    async fn a_bundle_that_declares_no_hosts_is_refused_an_uncheckable_node_too() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "silentagent",
            json!([]),
            "n.ai.agent",
            json!({}),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "silentagent").await;
        let error = payload["error"].as_str().unwrap_or_default();
        assert!(
            error.contains("FW_EGRESS_UNCHECKED_NODE")
                && error.contains("'n.ai.agent'")
                && error.contains("node bundle 'silentagent'"),
            "silence buys nothing, got: {error}"
        );
    }

    /// Both curated bundles compose `n.script`, so a guard that refused it
    /// under the shipped sandbox would break what ships. It does not: the
    /// sandbox denies `fetch`, so there is no egress to read.
    #[tokio::test]
    async fn a_script_runs_inside_a_bundle_under_the_shipped_sandbox() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "scriptpkg",
            json!(["api.telegram.org"]),
            "n.script",
            json!({ "source": "return { ok: true };" }),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "scriptpkg").await;
        assert!(
            !payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .contains("FW_EGRESS"),
            "no egress guard has anything to say about a sandbox that cannot fetch, got: {payload}"
        );
        assert_eq!(
            payload,
            json!({ "ok": true }),
            "the script ran to its own result, so this is not a refusal in disguise"
        );
    }

    /// The scope line: `spec.hosts` is a bundle's declaration about itself, and
    /// says nothing about the pipelines a user writes in their own project.
    #[tokio::test]
    async fn a_projects_own_request_is_not_governed_by_an_installed_bundle() {
        let root = tempfile::tempdir().expect("temp root");
        let platform = platform_for(root.path());
        write_bundle(
            root.path(),
            "installed",
            json!(["declared.invalid"]),
            "n.http.request",
            http_get("https://declared.invalid/x"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        // The same host the installed bundle would be refused for, reached from
        // the project's own graph, with no bundle policy in force.
        let graph: crate::pipeline::PipelineGraph = serde_json::from_value(json!({
            "id": "project-own",
            "entry_nodes": ["trigger"],
            "nodes": [
                {
                    "id": "trigger",
                    "kind": "n.trigger.function",
                    "input_pins": [],
                    "output_pins": ["out"],
                    "config": {}
                },
                {
                    "id": "call",
                    "kind": "n.http.request",
                    "input_pins": ["in"],
                    "output_pins": ["out"],
                    "config": http_get("https://elsewhere.invalid/x")
                }
            ],
            "edges": [
                { "from_node": "trigger", "from_pin": "out", "to_node": "call", "to_pin": "in" }
            ]
        }))
        .expect("project graph");

        let engine = super::BasicPipelineEngine::new(
            Arc::new(platform.project_sandbox(OWNER, PROJECT)),
            crate::rwe::resolve_engine_or_default(None),
            Some(platform.credentials.clone()),
        )
        .with_platform(platform.clone())
        .with_data_root(platform.config.data_root.clone());

        let ctx = crate::pipeline::PipelineContext {
            owner: OWNER.to_string(),
            project: PROJECT.to_string(),
            pipeline: "project-own".to_string(),
            request_id: "test".to_string(),
            route: Default::default(),
            input: json!({}),
            trigger: None,
            placeholder: None,
        };
        let error =
            crate::pipeline::interface::PipelineEngine::execute_async(&engine, &graph, &ctx)
                .await
                .expect_err("the host does not resolve, so the run still fails");
        assert_ne!(
            error.code, "FW_EGRESS_UNDECLARED_HOST",
            "a project's own request is the user's own choice: {}",
            error.message
        );
    }
}
