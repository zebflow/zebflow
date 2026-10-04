//! Execution host for nodes provided by bundles: the official composites
//! shipped in the binary (`telegram.*`, `ai.embedding.generate`) and installed
//! ones (`x.<package>.<noun>.<verb>`).
//!
//! A bundle-provided node is dispatched from its package manifest, never from
//! its kind, so the same node may be composite or WASM without the graph
//! knowing. Composite behaviour runs an inner function pipeline; WASM behaviour
//! runs a declared export through [`super::wasm_host`].
//!
//! # What a bundle node answers (`node-conventions.md` §1, §6)
//!
//! The engine places the answer, not the bundle, so a composite answers the
//! way a native node of the same kind would:
//!
//! - An acting node adds **one key, its noun** ([`answer_key`]:
//!   `telegram.message.send` → `message`, `x.acme.invoice.create` →
//!   `invoice`), holding its result, and keeps the payload it was given.
//! - A composite's result is **its function's last node's answer**
//!   ([`PipelineOutput::function_result`], the rule `function.result.call`
//!   shares): the value the last node that answered added under its own key,
//!   or — when that node has no key of its own — the final payload without
//!   the function trigger's `function` key. A function therefore ends in the
//!   node whose answer is the result, usually a `javascript.script.run` that
//!   shapes it (`script` → the noun).
//!
//! [`PipelineOutput::function_result`]: crate::pipeline::model::PipelineOutput::function_result
//! - A WASM node's result is what its export returned.
//! - A failed function delivers `error` with
//!   `<noun>: { ok: false, error: { code, message } }`, the payload kept.
//! - A trigger answers its source (`trigger.telegram` → `telegram`): its
//!   handler's result, or — without a handler, or when the handler fails —
//!   what the ingress delivered, so an event is never dropped.
//!
//! # What a composite's function receives
//!
//! Its flags and nothing of the payload (§3, every source is explicit): each
//! config key the node declares (a flag's `config_key` or a
//! `config_schema` property), as the engine resolved it, except the
//! credential keys — a credential becomes the function's `$placeholder`
//! values instead. `trigger.function` answers them under `function`, so the
//! function reads `input.function.<config key>`. Before it runs, a required
//! flag that resolved empty, a word outside a closed choice, a config off its
//! provider's profile and a credential of another provider's kind are
//! refused with `FW_NODE_PACKAGE_CONFIG`.
//!
//! Package discovery and validation live in
//! `src/platform/services/node_registry.rs`.

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::pipeline::PipelineContext;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{NodeDefinition, PipelineError};
use crate::pipeline::nodes::basic::trigger::answer_under;
use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, answer_key};
use crate::pipeline::security::BundleEgress;
use crate::platform::model::NodePackageManifest;

use super::basic::BasicPipelineEngine;

/// The key a bundle node answers under: its noun, or a trigger's source. A
/// kind of no known shape (refused at registration) answers `result`.
fn answer_key_of(kind: &str) -> String {
    answer_key(kind).unwrap_or_else(|| "result".to_string())
}

/// `{ <key>: value }` added to the payload the node was given.
fn answered(arriving: &Value, key: &str, value: Value) -> Value {
    let mut answer = Map::new();
    answer.insert(key.to_string(), value);
    with_answer(arriving, Value::Object(answer))
}

fn scope_of(input: &NodeExecutionInput) -> (String, String) {
    let text = |key: &str| {
        input.metadata.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    (text("owner"), text("project"))
}

pub(super) async fn execute_installed_node(
    kind: String,
    config: Value,
    platform: Arc<crate::platform::services::PlatformService>,
    parent_egress: Option<Arc<BundleEgress>>,
    input: NodeExecutionInput,
) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
    use crate::platform::model::NodePackageSource;

    let (owner, project) = scope_of(&input);
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
    let key = answer_key_of(&kind);

    if manifest.trigger.is_some() {
        let output =
            execute_installed_trigger(&kind, &key, &manifest, &config, &platform, egress, input)
                .await;
        return Ok(vec![output]);
    }

    check_declared_flags(&kind, &manifest.definition, &config)?;
    check_provider_profile(&kind, &manifest.definition, &config, &platform, &owner, &project)?;

    match manifest.source {
        NodePackageSource::Wasm => {
            let arriving = input.payload.clone();
            let outputs =
                super::wasm_host::execute_wasm_node(kind, config, platform, input).await?;
            Ok(outputs
                .into_iter()
                .map(|output| NodeExecutionOutput {
                    payload: answered(&arriving, &key, output.payload),
                    ..output
                })
                .collect())
        }
        NodePackageSource::Composite => {
            let output =
                execute_composite_node(&kind, &key, &manifest, &config, &platform, egress, input)
                    .await;
            Ok(vec![output])
        }
        NodePackageSource::Declarative => Err(PipelineError::new(
            "FW_NODE_PACKAGE_NOT_EXECUTABLE",
            format!("node '{kind}' declares no run binding and is not a trigger"),
        )),
    }
}

/// A bundle node's resolved flags against its own definition: a required flag
/// may not resolve empty, and a closed choice takes only its words — the rule
/// a native node enforces in its own `Config`, enforced here for every bundle.
fn check_declared_flags(
    kind: &str,
    definition: &NodeDefinition,
    config: &Value,
) -> Result<(), PipelineError> {
    for flag in &definition.dsl_flags {
        let value = config.get(&flag.config_key).unwrap_or(&Value::Null);
        let empty = match value {
            Value::Null => true,
            Value::String(text) => text.trim().is_empty(),
            Value::Array(items) => items.is_empty(),
            Value::Object(map) => map.is_empty(),
            _ => false,
        };
        if empty {
            if flag.required {
                return Err(PipelineError::new(
                    "FW_NODE_PACKAGE_CONFIG",
                    format!("{kind}: {} is empty; it needs a value", flag.flag),
                ));
            }
            continue;
        }
        if flag.choices.is_empty() {
            continue;
        }
        let words: Vec<&Value> = match value {
            Value::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        for word in words {
            let text = match word {
                Value::String(text) => text.trim().to_string(),
                other => other.to_string(),
            };
            if !flag.choices.iter().any(|choice| *choice == text) {
                return Err(PipelineError::new(
                    "FW_NODE_PACKAGE_CONFIG",
                    format!(
                        "{kind}: {} '{text}' is not one of {}",
                        flag.flag,
                        flag.choices.join(", ")
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// A bundle node with provider profiles (`node-conventions.md` §11): its
/// resolved config against the chosen provider's profile, and the credential
/// it names against the kinds that provider's key lives in — the rule a
/// native node holds in its own build, held here for every bundle.
fn check_provider_profile(
    kind: &str,
    definition: &NodeDefinition,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    owner: &str,
    project: &str,
) -> Result<(), PipelineError> {
    use crate::pipeline::nodes::shared::profile::{CREDENTIAL_FLAG, check_credential, check_profile, chosen_profile};
    if definition.profiles.is_empty() {
        return Ok(());
    }
    let refused = |message: String| PipelineError::new("FW_NODE_PACKAGE_CONFIG", format!("{kind}: {message}"));
    check_profile(definition, config).map_err(refused)?;
    let provider = chosen_profile(definition, config).map_err(refused)?.provider.clone();
    let Some(key) = definition.dsl_flags.iter().find(|f| f.flag == CREDENTIAL_FLAG).map(|f| f.config_key.as_str()) else {
        return Ok(());
    };
    let Some(id) = config.get(key).and_then(Value::as_str).map(str::trim).filter(|id| !id.is_empty()) else {
        return Ok(());
    };
    if let Ok(Some(credential)) = platform.credentials.get_project_credential(owner, project, id) {
        check_credential(definition, &provider, id, &credential.kind).map_err(refused)?;
    }
    Ok(())
}

/// What a composite's function receives: every config key the node declares,
/// as resolved, without the credential keys (those are `$placeholder`s).
fn function_arguments(manifest: &NodePackageManifest, config: &Value) -> Value {
    let credential_keys: Vec<&str> =
        manifest.credentials.iter().map(|credential| credential.config_key.as_str()).collect();
    let mut declared: Vec<&str> =
        manifest.definition.dsl_flags.iter().map(|flag| flag.config_key.as_str()).collect();
    if let Some(properties) =
        manifest.definition.config_schema.get("properties").and_then(Value::as_object)
    {
        declared.extend(properties.keys().map(String::as_str));
    }
    let mut arguments = Map::new();
    for key in declared {
        if credential_keys.contains(&key) || arguments.contains_key(key) {
            continue;
        }
        if let Some(value) = config.get(key) {
            arguments.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(arguments)
}

/// An engine for one of a bundle's function pipelines, under its egress.
fn bundle_engine(
    owner: &str,
    project: &str,
    platform: &Arc<crate::platform::services::PlatformService>,
    egress: Option<Arc<BundleEgress>>,
) -> BasicPipelineEngine {
    BasicPipelineEngine::new(
        Arc::new(platform.project_sandbox(owner, project)),
        crate::rwe::resolve_engine_or_default(None),
        Some(platform.credentials.clone()),
    )
    .with_platform(platform.clone())
    .with_ws_hub(platform.ws_hub.clone())
    .with_state_bus(platform.state_bus.clone())
    .with_data_root(platform.config.data_root.clone())
    .with_bundle_egress(egress)
}

fn run_context(
    owner: &str,
    project: &str,
    pipeline: String,
    request_prefix: &str,
    input: Value,
    placeholder_map: Map<String, Value>,
) -> PipelineContext {
    PipelineContext {
        owner: owner.to_string(),
        project: project.to_string(),
        request_id: format!(
            "{request_prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        pipeline,
        route: Default::default(),
        input,
        trigger: None,
        placeholder: if placeholder_map.is_empty() {
            None
        } else {
            Some(json!(placeholder_map))
        },
    }
}

/// Runs a trigger's inbound handler for one event and answers under the
/// trigger's source. A trigger never drops an event: with no handler, or a
/// handler that fails, the source holds what the ingress delivered.
async fn execute_installed_trigger(
    kind: &str,
    source: &str,
    manifest: &NodePackageManifest,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    egress: Option<Arc<BundleEgress>>,
    input: NodeExecutionInput,
) -> NodeExecutionOutput {
    let (owner, project) = scope_of(&input);
    let delivered = |trace: String| NodeExecutionOutput {
        output_pins: vec!["out".to_string()],
        payload: answer_under(source, input.payload.clone()),
        trace: vec![trace],
    };
    let handled = |value: Value, trace: String| NodeExecutionOutput {
        output_pins: vec!["out".to_string()],
        payload: answer_under(source, value),
        trace: vec![trace],
    };
    // The handler transforms the event: a webhook's body, else the payload.
    let event = input.payload.get("body").cloned().unwrap_or_else(|| input.payload.clone());

    // The inbound handler is the node's run binding, so a trigger may be
    // implemented as a composite function or as a WASM export.
    let Some(binding) = manifest.run.as_ref() else {
        return delivered(format!("trigger '{kind}' has no handler, delivered as received"));
    };

    if binding.module.is_some() {
        let Some(installed) = platform.node_registry.get_by_kind(&owner, &project, kind) else {
            return delivered(format!("wasm trigger '{kind}' package not installed, delivered as received"));
        };
        let Some((module_spec, export)) = installed.manifest.wasm_target() else {
            return delivered(format!("wasm trigger '{kind}' has no resolvable export, delivered as received"));
        };
        return match crate::pipeline::engines::wasm_host::run_wasm_export(
            kind,
            &installed.package_dir,
            module_spec,
            export,
            config,
            &event,
            &input.metadata,
        ) {
            Ok(value) => handled(value, format!("wasm trigger '{kind}' export '{export}' ok")),
            Err(err) => {
                eprintln!(
                    "wasm_trigger: export '{export}' failed for '{kind}': {}, delivering as received",
                    err.message
                );
                delivered(format!("wasm trigger '{kind}' export error: {}", err.message))
            }
        };
    }

    let Some(fn_name) = binding.function.as_deref() else {
        return delivered(format!("trigger '{kind}' has no handler, delivered as received"));
    };
    let graph = match platform.node_registry.load_composite_function(
        &owner,
        &project,
        kind,
        Some(fn_name),
    ) {
        Ok(graph) => graph,
        Err(e) => {
            eprintln!(
                "composite_trigger: handler '{fn_name}' load failed for '{kind}': {}, delivering as received",
                e.message
            );
            return delivered(format!("composite trigger '{kind}' handler load error"));
        }
    };
    let placeholder_map = build_composite_placeholder_map(kind, config, &owner, &project, platform);
    let ctx = run_context(
        &owner,
        &project,
        format!("composite_trigger::{kind}::{fn_name}"),
        &format!("ct-{kind}"),
        event,
        placeholder_map,
    );
    match bundle_engine(&owner, &project, platform, egress).execute_async(&graph, &ctx).await {
        Ok(output) => handled(
            output.function_result(),
            format!("composite trigger '{kind}' handler ok"),
        ),
        Err(e) => {
            eprintln!(
                "composite_trigger: handler '{fn_name}' failed for '{kind}': {}, delivering as received",
                e.message
            );
            delivered(format!("composite trigger '{kind}' handler error: {}", e.message))
        }
    }
}

/// Runs a composite node's function and answers its result under the noun.
async fn execute_composite_node(
    kind: &str,
    key: &str,
    manifest: &NodePackageManifest,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    egress: Option<Arc<BundleEgress>>,
    input: NodeExecutionInput,
) -> NodeExecutionOutput {
    let (owner, project) = scope_of(&input);
    let failed = |code: &str, message: &str, trace: String| NodeExecutionOutput {
        output_pins: vec!["error".to_string()],
        payload: answered(
            &input.payload,
            key,
            json!({ "ok": false, "error": { "code": code, "message": message } }),
        ),
        trace: vec![trace],
    };

    let graph = match platform.node_registry.load_composite_pipeline(&owner, &project, kind) {
        Ok(graph) => graph,
        Err(e) => {
            return failed(&e.code, &e.message, format!("composite '{kind}' load error: {}", e.message));
        }
    };

    let placeholder_map = build_composite_placeholder_map(kind, config, &owner, &project, platform);
    let ctx = run_context(
        &owner,
        &project,
        format!("composite::{kind}"),
        &format!("composite-{kind}"),
        function_arguments(manifest, config),
        placeholder_map,
    );

    match bundle_engine(&owner, &project, platform, egress).execute_async(&graph, &ctx).await {
        Ok(output) => NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: answered(&input.payload, key, output.function_result()),
            trace: vec![format!("composite '{kind}' ok")],
        },
        Err(e) => failed(
            &e.code,
            &e.message,
            format!("composite '{kind}' error: {} — {}", e.code, e.message),
        ),
    }
}

/// Builds a placeholder name → resolved value map from a composite node's
/// credential declarations and the user's config.
///
/// For each credential declaration in the manifest:
/// 1. Read `config_key` to find which config field holds the credential ID.
/// 2. Fetch the credential from `CredentialService`.
/// 3. For each `placeholders` entry (e.g. `"BOT_TOKEN" -> "token"`), read the
///    secret field and map the placeholder name to the actual value.
///
/// The map is the inner pipeline's `$placeholder` scope, so
/// `{{ $placeholder.X }}` resolves to the credential's value there.
pub fn build_composite_placeholder_map(
    kind: &str,
    config: &Value,
    owner: &str,
    project: &str,
    platform: &Arc<crate::platform::services::PlatformService>,
) -> Map<String, Value> {
    let mut placeholder_map = Map::new();

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

/// Where the value a webhook trigger's `secret_header` names lives:
/// `(credential id, secret field)` of the node's credential.
fn trigger_secret_source(
    manifest: &NodePackageManifest,
    config: &Value,
    placeholder: &str,
) -> Option<(String, String)> {
    manifest.credentials.iter().find_map(|credential| {
        let field = credential.placeholders.get(placeholder)?;
        let id = config.get(&credential.config_key)?.as_str()?.trim();
        (!id.is_empty()).then(|| (id.to_string(), field.clone()))
    })
}

/// Refuses an inbound request to a webhook trigger whose sender must prove
/// itself (`trigger.secret_header`) and did not: the header is compared in
/// constant time with the node's credential value. A trigger that declares
/// no secret header passes. `header` reads one request header by name.
pub fn check_trigger_secret(
    platform: &Arc<crate::platform::services::PlatformService>,
    owner: &str,
    project: &str,
    kind: &str,
    config: &Value,
    header: &dyn Fn(&str) -> Option<String>,
) -> Result<(), PipelineError> {
    use subtle::ConstantTimeEq;

    let Some(manifest) = platform.node_registry.get_manifest(owner, project, kind) else {
        return Ok(());
    };
    let Some(secret) = manifest.trigger.as_ref().and_then(|t| t.secret_header.as_ref()) else {
        return Ok(());
    };
    let expected = build_composite_placeholder_map(kind, config, owner, project, platform)
        .get(&secret.placeholder)
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    if expected.is_empty() {
        return Err(PipelineError::new(
            "FW_WEBHOOK_SECRET_UNSET",
            format!(
                "{kind}: the credential holds no {}; activate the pipeline again to generate it",
                secret.placeholder
            ),
        ));
    }
    let Some(given) = header(&secret.header).filter(|value| !value.is_empty()) else {
        return Err(PipelineError::new(
            "FW_WEBHOOK_SECRET_MISSING",
            format!("{kind}: the request carries no {} header", secret.header),
        ));
    };
    if !bool::from(given.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(PipelineError::new(
            "FW_WEBHOOK_SECRET_MISMATCH",
            format!("{kind}: the {} header does not match", secret.header),
        ));
    }
    Ok(())
}

/// Before a webhook trigger with a `secret_header` is activated, makes sure
/// its credential holds the secret: 32 random bytes, hex, written into the
/// credential when the field is empty. The activation hook then hands it to
/// the sender (Telegram's `setWebhook` `secret_token`).
pub fn ensure_trigger_secret(
    platform: &Arc<crate::platform::services::PlatformService>,
    owner: &str,
    project: &str,
    kind: &str,
    config: &Value,
) -> Result<(), String> {
    use rand::RngExt as _;

    let Some(manifest) = platform.node_registry.get_manifest(owner, project, kind) else {
        return Ok(());
    };
    let Some(secret) = manifest.trigger.as_ref().and_then(|t| t.secret_header.as_ref()) else {
        return Ok(());
    };
    let Some((credential_id, field)) = trigger_secret_source(&manifest, config, &secret.placeholder)
    else {
        return Err(format!("{kind}: no credential is configured to hold {}", secret.placeholder));
    };
    let credential = platform
        .credentials
        .get_project_credential(owner, project, &credential_id)
        .map_err(|e| e.message)?
        .ok_or_else(|| format!("{kind}: credential '{credential_id}' not found"))?;
    let present = credential
        .secret
        .get(&field)
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    if present {
        return Ok(());
    }
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let mut stored = credential.secret.clone();
    if !stored.is_object() {
        stored = json!({});
    }
    stored[field.as_str()] = Value::String(hex::encode(bytes));
    platform
        .credentials
        .upsert_project_credential(
            owner,
            project,
            &crate::platform::model::UpsertProjectCredentialRequest {
                credential_id: credential.credential_id.clone(),
                title: credential.title.clone(),
                kind: credential.kind.clone(),
                secret: stored,
                notes: credential.notes.clone(),
            },
        )
        .map(|_| ())
        .map_err(|e| e.message)
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
        json!({ "method": "GET", "url": url, "parse": "json" })
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
                    "kind": format!("x.{}.result.call", slug),
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
                    "kind": "trigger.function",
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

    /// A failed bundle node's `code: message`, from the `{ ok: false, error }`
    /// it answers under its noun (`result` for these `x.<slug>.result.call`
    /// fixtures); empty when it did not fail.
    fn failure(payload: &Value) -> String {
        let error = &payload["result"]["error"];
        match (error["code"].as_str(), error["message"].as_str()) {
            (Some(code), Some(message)) => format!("{code}: {message}"),
            _ => String::new(),
        }
    }

    async fn run_bundle_node(platform: &Arc<PlatformService>, slug: &str) -> Value {
        let outputs = super::execute_installed_node(
            format!("x.{slug}.result.call"),
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
            "http.response.fetch",
            http_get("https://elsewhere.invalid/steal"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "undeclared").await;
        assert_eq!(
            failure(&payload),
            "FW_EGRESS_UNDECLARED_HOST: http.response.fetch outbound host 'elsewhere.invalid' is \
             not declared by node bundle 'undeclared' (spec.hosts: declared.invalid)",
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
            "http.response.fetch",
            http_get("https://declared.invalid/embed"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "declaredhost").await;
        let error = failure(&payload);
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
            "http.response.fetch",
            http_get("https://anywhere.invalid/x"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "nohosts").await;
        let error = failure(&payload);
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
            "x.inner.result.call",
            json!({}),
        );
        write_bundle(
            root.path(),
            "inner",
            json!(["api.inner.invalid"]),
            "http.response.fetch",
            http_get("https://api.inner.invalid/x"),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundles register");

        // Run directly, the inner bundle reaches its own declared host.
        let payload = run_bundle_node(&platform, "inner").await;
        let error = failure(&payload);
        assert!(
            !error.contains("FW_EGRESS_UNDECLARED_HOST"),
            "the inner bundle's own host is fine on its own, got: {error}"
        );

        // Reached through the outer bundle, the same request answers to the
        // outer declaration too, which does not name that host.
        let payload = run_bundle_node(&platform, "outer").await;
        let error = failure(&payload);
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
            "ai.text.generate",
            json!({}),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "agentpkg").await;
        let error = failure(&payload);
        assert!(
            error.contains("FW_EGRESS_UNCHECKED_NODE")
                && error.contains("'ai.text.generate'")
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
            "ai.text.generate",
            json!({}),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "silentagent").await;
        let error = failure(&payload);
        assert!(
            error.contains("FW_EGRESS_UNCHECKED_NODE")
                && error.contains("'ai.text.generate'")
                && error.contains("node bundle 'silentagent'"),
            "silence buys nothing, got: {error}"
        );
    }

    /// Both curated bundles compose `javascript.script.run`, so a guard that refused it
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
            "javascript.script.run",
            json!({ "source": "return { ok: true };" }),
        );
        platform
            .node_registry
            .refresh_project(OWNER, PROJECT)
            .expect("bundle registers");

        let payload = run_bundle_node(&platform, "scriptpkg").await;
        assert!(
            !failure(&payload).contains("FW_EGRESS"),
            "no egress guard has anything to say about a sandbox that cannot fetch, got: {payload}"
        );
        assert_eq!(
            payload["result"],
            json!({ "ok": true }),
            "the script ran to its own result, so this is not a refusal in disguise: {payload}"
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
            "http.response.fetch",
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
                    "kind": "trigger.function",
                    "input_pins": [],
                    "output_pins": ["out"],
                    "config": {}
                },
                {
                    "id": "call",
                    "kind": "http.response.fetch",
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

/// What a bundle node answers, through the shipped bundles' own definitions
/// and functions (`node-conventions.md` §1, §6).
///
/// The official bundles call fixed hosts, so each is installed here as a
/// project copy (`x.<copy>.*`) whose `http.response.fetch` nodes are swapped
/// for `x.tgstub.response.fetch`: a composite that answers `response` like
/// the real node, from a canned Telegram or embeddings reply, and echoes the
/// request it was given. Everything else — the flags, the functions, the
/// placement of the answer — is the shipped bundle's.
#[cfg(test)]
mod answer_tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use serde_json::{Value, json};

    use crate::pipeline::nodes::NodeExecutionInput;
    use crate::pipeline::nodes::shared::test_platform::{TestPlatform, test_platform};
    use crate::platform::services::PlatformService;

    const OWNER: &str = "superadmin";
    const PROJECT: &str = "default";
    /// 2026-01-01T00:00:00Z, the stub's send time.
    const SENT: i64 = 1_767_225_600;

    const TELEGRAM_DEFINITION: &str = include_str!("../nodes/bundled/telegram/definition.json");
    const TELEGRAM_FUNCTIONS: &[(&str, &str)] = &[
        ("send-message", include_str!("../nodes/bundled/telegram/functions/send-message.zf.json")),
        ("edit-message", include_str!("../nodes/bundled/telegram/functions/edit-message.zf.json")),
        ("register-webhook", include_str!("../nodes/bundled/telegram/functions/register-webhook.zf.json")),
        ("delete-webhook", include_str!("../nodes/bundled/telegram/functions/delete-webhook.zf.json")),
        ("transform-update", include_str!("../nodes/bundled/telegram/functions/transform-update.zf.json")),
    ];


    fn nodes_dir(platform: &PlatformService) -> PathBuf {
        platform
            .projects
            .project_layout(OWNER, PROJECT)
            .expect("project layout")
            .data_hub_nodes_dir()
    }

    fn write(path: PathBuf, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        std::fs::write(path, bytes).expect("write");
    }

    /// A bundle's JSON text, with every real HTTP call made through the stub.
    fn stubbed(function: &str) -> String {
        function.replace("\"http.response.fetch\"", "\"x.tgstub.response.fetch\"")
    }

    /// The HTTP stub bundle (`tests/fixtures/contracts/node-bundle/http-stub`):
    /// `x.tgstub.response.fetch` answers a Telegram Bot API reply for a message
    /// method (chat `refuse` is refused as Telegram refuses an unknown chat),
    /// an embeddings reply for `/embeddings`, `true` for anything else, and
    /// echoes the request.
    fn install_stub(platform: &PlatformService) {
        let dir = nodes_dir(platform).join("tgstub");
        write(dir.join("definition.json"), include_bytes!("../../../tests/fixtures/contracts/node-bundle/http-stub/definition.json"));
        write(
            dir.join("functions/reply.zf.json"),
            include_bytes!("../../../tests/fixtures/contracts/node-bundle/http-stub/functions/reply.zf.json"),
        );
        write(dir.join("icon.svg"), include_bytes!("../../../tests/fixtures/contracts/node-bundle/http-stub/icon.svg"));
    }

    /// The shipped telegram bundle as project bundle `tgcopy`: the same flags,
    /// functions and answers under `x.tgcopy.*`.
    fn install_telegram_copy(platform: &PlatformService) {
        let dir = nodes_dir(platform).join("tgcopy");
        let definition = TELEGRAM_DEFINITION
            .replace("\"trigger.telegram\"", "\"x.tgcopy.telegram.receive\"")
            .replace("\"telegram.message.send\"", "\"x.tgcopy.message.send\"")
            .replace("\"telegram.message.edit\"", "\"x.tgcopy.message.edit\"")
            .replace("\"name\": \"telegram\"", "\"name\": \"tgcopy\"")
            .replace("\"package\": \"telegram\"", "\"package\": \"tgcopy\"")
            .replace("telegram_bot", "tgcopy_bot");
        write(dir.join("definition.json"), definition.as_bytes());
        for (name, function) in TELEGRAM_FUNCTIONS {
            write(dir.join(format!("functions/{name}.zf.json")), stubbed(function).as_bytes());
        }
        for icon in ["icon.svg", "icons/trigger.svg", "icons/message-send.svg", "icons/message-edit.svg"] {
            write(dir.join(icon), b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
        }
    }

    /// A bot credential with a generated, obviously fake token.
    fn bot_credential(platform: &PlatformService) -> (String, String) {
        let token = format!("{}:{}", std::process::id(), "fake".repeat(9));
        let secret = format!("hook-{}", "0".repeat(16));
        platform
            .credentials
            .upsert_project_credential(
                OWNER,
                PROJECT,
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "bot".to_string(),
                    title: "Bot".to_string(),
                    kind: "tgcopy_bot".to_string(),
                    secret: json!({ "token": token, "webhook_secret": secret }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        (token, secret)
    }

    struct Bundles {
        platform: TestPlatform,
        token: String,
        webhook_secret: String,
    }

    fn telegram() -> Bundles {
        let platform = test_platform();
        install_stub(&platform);
        install_telegram_copy(&platform);
        platform.node_registry.refresh_project(OWNER, PROJECT).expect("bundles register");
        let (token, webhook_secret) = bot_credential(&platform);
        Bundles { platform, token, webhook_secret }
    }

    async fn run(
        platform: &Arc<PlatformService>,
        kind: &str,
        config: Value,
        payload: Value,
    ) -> Result<(Vec<String>, Value), crate::pipeline::model::PipelineError> {
        let outputs = super::execute_installed_node(
            kind.to_string(),
            config,
            platform.clone(),
            None,
            NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload,
                metadata: json!({ "owner": OWNER, "project": PROJECT }),
                bus: None,
            },
        )
        .await?;
        let output = outputs.into_iter().next().expect("one emission");
        Ok((output.output_pins, output.payload))
    }

    fn keys(payload: &Value) -> Vec<&str> {
        let mut keys: Vec<&str> = payload.as_object().expect("object").keys().map(String::as_str).collect();
        keys.sort();
        keys
    }

    #[test]
    fn a_custom_composite_answers_under_its_noun() {
        use crate::pipeline::nodes::answer_key;
        assert_eq!(answer_key("x.acme.invoice.create").as_deref(), Some("invoice"));
        assert_eq!(answer_key("x.acme.invoice"), None, "not a custom composite's shape");
        assert_eq!(answer_key("telegram.message.send").as_deref(), Some("message"));
        assert_eq!(answer_key("trigger.telegram").as_deref(), Some("telegram"));
        assert_eq!(answer_key("ai.embedding.generate").as_deref(), Some("embedding"));
    }

    /// One key is added, the noun, holding the function's last node's
    /// answer; the arriving payload is kept and never reaches the function.
    #[tokio::test]
    async fn a_composite_answer_lands_under_its_noun_with_the_payload_kept() {
        let bundles = telegram();
        let (pins, payload) = run(
            &bundles.platform,
            "x.tgstub.response.fetch",
            json!({ "url": "https://api.telegram.org/botX/getMe", "body": { "a": 1 } }),
            json!({ "kept": 1, "webhook": { "body": "untouched" } }),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"]);
        assert_eq!(keys(&payload), ["kept", "response", "webhook"]);
        assert_eq!(payload["kept"], 1);
        assert_eq!(payload["webhook"], json!({ "body": "untouched" }));
        assert_eq!(
            payload["response"]["body"]["echo"],
            json!({ "method": "getMe", "body": { "a": 1 } }),
            "the function received its flags, and nothing of the payload"
        );
    }

    #[tokio::test]
    async fn telegram_sends_text_and_answers_message() {
        let bundles = telegram();
        let (pins, payload) = run(
            &bundles.platform,
            "x.tgcopy.message.send",
            json!({
                "credential_id": "bot",
                "recipient": 1001,
                "text": "<b>hello</b>",
                "format": "html",
                "keyboard": { "Yes": "y", "No": "n" },
                "in_reply_to": 7,
                "silent": true
            }),
            json!({ "kept": true }),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"], "{payload}");
        assert_eq!(keys(&payload), ["kept", "message"]);
        let message = &payload["message"];
        assert_eq!(message["id"], 42);
        assert_eq!(message["recipient"], 1001);
        assert_eq!(message["sent_at"], "2026-01-01T00:00:00.000Z");
        assert_eq!(message["telegram"]["chat"]["id"], 1001);
        assert_eq!(
            message["telegram"]["stub_request"],
            json!({ "method": "sendMessage", "body": {
                "chat_id": "1001",
                "text": "<b>hello</b>",
                "parse_mode": "HTML",
                "reply_markup": { "inline_keyboard": [
                    [{ "text": "Yes", "callback_data": "y" }],
                    [{ "text": "No", "callback_data": "n" }]
                ] },
                "disable_notification": true,
                "reply_to_message_id": 7
            } })
        );
        assert!(!payload.to_string().contains(&bundles.token), "the bot token never reaches the answer");
    }

    #[tokio::test]
    async fn telegram_sends_an_image_or_a_file_with_text_as_the_caption() {
        let bundles = telegram();
        for (flag, value, method, field) in [
            ("image", "https://example.com/cat.jpg", "sendPhoto", "photo"),
            ("file", "file-id-1", "sendDocument", "document"),
        ] {
            let (pins, payload) = run(
                &bundles.platform,
                "x.tgcopy.message.send",
                json!({ "credential_id": "bot", "recipient": "@demo", "text": "a caption", flag: value, "format": "markdown" }),
                json!({}),
            )
            .await
            .expect("runs");
            assert_eq!(pins, ["out"], "{payload}");
            assert_eq!(
                payload["message"]["telegram"]["stub_request"],
                json!({ "method": method, "body": {
                    "chat_id": "@demo", field: value, "caption": "a caption", "parse_mode": "MarkdownV2"
                } })
            );
            assert_eq!(payload["message"]["recipient"], "@demo");
        }
    }

    #[tokio::test]
    async fn telegram_refusals_answer_under_message_on_the_error_pin() {
        let bundles = telegram();
        let (pins, payload) = run(
            &bundles.platform,
            "x.tgcopy.message.send",
            json!({ "credential_id": "bot", "recipient": 1, "text": "t", "image": "https://example.com/a.jpg", "file": "f" }),
            json!({ "kept": 1 }),
        )
        .await
        .expect("delivers");
        assert_eq!(pins, ["error"]);
        assert_eq!(keys(&payload), ["kept", "message"]);
        assert_eq!(payload["message"]["ok"], false);
        assert!(payload["message"]["error"]["message"].as_str().unwrap_or_default().contains("not both"), "{payload}");

        let (pins, payload) = run(
            &bundles.platform,
            "x.tgcopy.message.send",
            json!({ "credential_id": "bot", "recipient": "refuse", "text": "t" }),
            json!({}),
        )
        .await
        .expect("delivers");
        assert_eq!(pins, ["error"]);
        assert!(
            payload["message"]["error"]["message"].as_str().unwrap_or_default().contains("chat not found"),
            "Telegram's own reason is kept: {payload}"
        );

        for (config, flag) in [
            (json!({ "credential_id": "bot", "recipient": 1, "text": "t", "format": "markdownv2" }), "--format"),
            (json!({ "credential_id": "bot", "recipient": 1, "text": " " }), "--text"),
        ] {
            let refused = run(&bundles.platform, "x.tgcopy.message.send", config, json!({}))
                .await
                .expect_err("refused before the function runs");
            assert_eq!(refused.code, "FW_NODE_PACKAGE_CONFIG");
            assert!(refused.message.contains(flag), "{}", refused.message);
        }
    }

    #[tokio::test]
    async fn telegram_edits_a_message_and_answers_message() {
        let bundles = telegram();
        let (pins, payload) = run(
            &bundles.platform,
            "x.tgcopy.message.edit",
            json!({ "credential_id": "bot", "recipient": 1001, "id": 42, "text": "edited", "keyboard": { "Undo": "u" } }),
            json!({ "kept": 1 }),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"], "{payload}");
        assert_eq!(keys(&payload), ["kept", "message"]);
        let message = &payload["message"];
        assert_eq!(message["id"], 42);
        assert_eq!(message["recipient"], 1001);
        assert_eq!(message["sent_at"], "2025-12-31T23:59:00.000Z");
        assert_eq!(message["edited_at"], "2026-01-01T00:00:00.000Z");
        assert_eq!(
            message["telegram"]["stub_request"],
            json!({ "method": "editMessageText", "body": {
                "chat_id": "1001", "message_id": 42, "text": "edited",
                "reply_markup": { "inline_keyboard": [[{ "text": "Undo", "callback_data": "u" }]] }
            } })
        );
    }

    /// The trigger answers its source and nothing else lands at the root.
    #[tokio::test]
    async fn the_telegram_trigger_answers_the_update_under_its_source() {
        let bundles = telegram();
        let update = json!({
            "update_id": 5,
            "message": {
                "message_id": 9, "date": SENT, "text": "hi",
                "chat": { "id": 1001, "type": "private" },
                "from": { "id": 77, "username": "demo", "first_name": "Demo" }
            }
        });
        let (pins, payload) = run(
            &bundles.platform,
            "x.tgcopy.telegram.receive",
            json!({ "credential_id": "bot" }),
            json!({ "body": update, "headers": { "content-type": "application/json" }, "method": "POST" }),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"]);
        assert_eq!(
            payload,
            json!({ "telegram": {
                "update_id": 5, "type": "message", "message_id": 9, "chat_id": 1001, "chat_type": "private",
                "from_id": 77, "from_username": "demo", "from_first_name": "Demo", "text": "hi", "date": SENT
            } })
        );
        assert_eq!(crate::pipeline::nodes::answer_key("trigger.telegram").as_deref(), Some("telegram"));
    }

    /// Runs the trigger's `register-webhook` hook the way activation does,
    /// and answers its final payload.
    async fn run_register_hook(platform: &Arc<PlatformService>, config: &Value) -> Value {
        let kind = "x.tgcopy.telegram.receive";
        let graph = platform
            .node_registry
            .load_composite_function(OWNER, PROJECT, kind, Some("register-webhook"))
            .expect("hook function");
        let placeholder = super::build_composite_placeholder_map(kind, config, OWNER, PROJECT, platform);
        let ctx = super::run_context(
            OWNER,
            PROJECT,
            graph.id.clone(),
            "lifecycle-test",
            json!({
                "node_id": "t", "node_kind": kind, "config": config, "hook": "on_activate",
                "pipeline": "pipelines/bot.zf.json", "owner": OWNER, "project": PROJECT,
                "platform": { "public_url": "https://hooks.example.com" }
            }),
            placeholder,
        );
        let egress = Some(Arc::new(crate::pipeline::security::BundleEgress::extend(None, "tgcopy", &["api.telegram.org".to_string()])));
        crate::pipeline::interface::PipelineEngine::execute_async(
            &super::bundle_engine(OWNER, PROJECT, platform, egress),
            &graph,
            &ctx,
        )
        .await
        .expect("the hook runs")
        .value
    }

    /// Activation's hook still registers the webhook: the URL the trigger's
    /// path template serves, the chosen events and the credential's secret.
    #[tokio::test]
    async fn the_register_webhook_hook_still_registers_the_trigger_route() {
        let bundles = telegram();
        let value = run_register_hook(
            &bundles.platform,
            &json!({ "credential_id": "bot", "event": ["message", "callback_query"] }),
        )
        .await;
        assert_eq!(
            value["response"]["body"]["echo"],
            json!({ "method": "setWebhook", "body": {
                "url": "https://hooks.example.com/wh/superadmin/default/tg/bot",
                "allowed_updates": ["message", "callback_query"],
                "secret_token": bundles.webhook_secret
            } })
        );
        assert_eq!(value["entry"]["key"], "telegram:webhook:bot");
    }

    fn secret_check(
        platform: &Arc<PlatformService>,
        credential: &str,
        given: Option<&str>,
    ) -> Result<(), crate::pipeline::model::PipelineError> {
        let given = given.map(str::to_string);
        super::check_trigger_secret(
            platform,
            OWNER,
            PROJECT,
            "x.tgcopy.telegram.receive",
            &json!({ "credential_id": credential }),
            &move |name: &str| {
                assert_eq!(name, "X-Telegram-Bot-Api-Secret-Token");
                given.clone()
            },
        )
    }

    /// An update is believed only with the secret Telegram was given: none or
    /// a wrong one is refused before any run, the right one passes.
    #[tokio::test]
    async fn a_telegram_update_must_carry_the_webhook_secret() {
        let bundles = telegram();
        let missing = secret_check(&bundles.platform, "bot", None).expect_err("no header");
        assert_eq!(missing.code, "FW_WEBHOOK_SECRET_MISSING");
        let forged = secret_check(&bundles.platform, "bot", Some("forged")).expect_err("wrong header");
        assert_eq!(forged.code, "FW_WEBHOOK_SECRET_MISMATCH");
        assert!(!forged.message.contains(&bundles.webhook_secret), "the refusal never repeats the secret");
        secret_check(&bundles.platform, "bot", Some(&bundles.webhook_secret)).expect("the right secret passes");
        // A trigger that declares no secret header is not asked for one.
        super::check_trigger_secret(&bundles.platform, OWNER, PROJECT, "x.tgcopy.message.send", &json!({}), &|_: &str| None)
            .expect("an action node has no inbound secret");
    }

    /// A credential without a secret gets one at activation, before the hook
    /// hands it to Telegram; until then every update is refused.
    #[tokio::test]
    async fn activation_generates_a_missing_webhook_secret_and_registers_it() {
        let bundles = telegram();
        let token = format!("{}:{}", std::process::id(), "fake".repeat(9));
        bundles
            .platform
            .credentials
            .upsert_project_credential(
                OWNER,
                PROJECT,
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "fresh".to_string(),
                    title: "Fresh bot".to_string(),
                    kind: "tgcopy_bot".to_string(),
                    secret: json!({ "token": token }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        let unset = secret_check(&bundles.platform, "fresh", Some("anything")).expect_err("no secret yet");
        assert_eq!(unset.code, "FW_WEBHOOK_SECRET_UNSET");

        let config = json!({ "credential_id": "fresh" });
        let stored = || {
            bundles
                .platform
                .credentials
                .get_project_credential(OWNER, PROJECT, "fresh")
                .expect("read")
                .expect("credential")
        };
        super::ensure_trigger_secret(&bundles.platform, OWNER, PROJECT, "x.tgcopy.telegram.receive", &config)
            .expect("generated");
        let credential = stored();
        let secret = credential.secret["webhook_secret"].as_str().expect("a secret").to_string();
        assert_eq!(secret.len(), 64, "32 random bytes, hex");
        assert!(secret.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(credential.secret["token"], json!(token), "the rest of the credential is kept");
        assert_eq!(credential.title, "Fresh bot");

        super::ensure_trigger_secret(&bundles.platform, OWNER, PROJECT, "x.tgcopy.telegram.receive", &config)
            .expect("idempotent");
        assert_eq!(stored().secret["webhook_secret"], json!(secret), "an existing secret is never replaced");

        let value = run_register_hook(&bundles.platform, &config).await;
        assert_eq!(value["response"]["body"]["echo"]["body"]["secret_token"], json!(secret));
        secret_check(&bundles.platform, "fresh", Some(&secret)).expect("the registered secret passes");
    }

    /// The shipped embedding bundle as project bundle `embedcopy`, every
    /// HTTP call through the stub.
    fn install_embedding_copy(platform: &PlatformService) {
        let dir = nodes_dir(platform).join("embedcopy");
        let definition = include_str!("../nodes/bundled/openai-embedding/definition.json")
            .replace("\"ai.embedding.generate\"", "\"x.embedcopy.embedding.generate\"")
            .replace("\"openai-embedding\"", "\"embedcopy\"");
        write(dir.join("definition.json"), definition.as_bytes());
        write(
            dir.join("functions/embed.zf.json"),
            stubbed(include_str!("../nodes/bundled/openai-embedding/functions/embed.zf.json")).as_bytes(),
        );
        for icon in ["icon.svg", "icons/embedding.svg"] {
            write(dir.join(icon), b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
        }
    }

    /// `--provider` is closed to the profiles, `--model` to the provider's
    /// models, and a credential of the other provider's kind is refused
    /// before the function runs.
    #[tokio::test]
    async fn the_embedding_holds_its_config_to_the_providers_profile() {
        let platform = test_platform();
        install_stub(&platform);
        install_embedding_copy(&platform);
        platform.node_registry.refresh_project(OWNER, PROJECT).expect("bundles register");
        platform
            .credentials
            .upsert_project_credential(
                OWNER,
                PROJECT,
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "router".to_string(),
                    title: "Router".to_string(),
                    kind: "openrouter".to_string(),
                    secret: json!({ "api_key": uuid::Uuid::new_v4().to_string() }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        for (config, says) in [
            (json!({ "provider": "cohere", "credential_id": "router", "text": ["a"] }), "'cohere' is not one of openai, openrouter"),
            (json!({ "provider": "openai", "credential_id": "none", "text": ["a"], "model": "nomic-embed-text" }), "takes --model text-embedding-3-small|text-embedding-3-large|text-embedding-ada-002"),
            (json!({ "provider": "openai", "credential_id": "none", "text": ["a"], "option": { "dimensions": "256" } }), "'dimensions' is not a setting of --provider openai"),
            (json!({ "provider": "openai", "credential_id": "router", "text": ["a"] }), "credential 'router' is kind 'openrouter'; --provider openai takes a credential of kind openai"),
        ] {
            let err = run(&platform, "x.embedcopy.embedding.generate", config, json!({})).await.err().expect("refused");
            assert_eq!(err.code, "FW_NODE_PACKAGE_CONFIG");
            assert!(err.message.contains(says), "{}", err.message);
        }
        let (pins, payload) = run(
            &platform,
            "x.embedcopy.embedding.generate",
            json!({ "provider": "openrouter", "credential_id": "router", "text": ["ab"] }),
            json!({}),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"], "{payload}");
        assert_eq!(payload["embedding"]["model"], "openai/text-embedding-3-small", "the provider's default model");
    }

    #[tokio::test]
    async fn the_embedding_answers_vectors_model_and_dims_under_embedding() {
        let platform = test_platform();
        install_stub(&platform);
        install_embedding_copy(&platform);
        platform.node_registry.refresh_project(OWNER, PROJECT).expect("bundles register");

        let (pins, payload) = run(
            &platform,
            "x.embedcopy.embedding.generate",
            json!({ "provider": "openai", "credential_id": "none", "text": ["ab", "abcde"] }),
            json!({ "kept": 1 }),
        )
        .await
        .expect("runs");
        assert_eq!(pins, ["out"], "{payload}");
        assert_eq!(keys(&payload), ["embedding", "kept"]);
        assert_eq!(
            payload["embedding"],
            json!({
                "vectors": [[2, 0.5, 0.25], [5, 0.5, 0.25]],
                "model": "text-embedding-3-small",
                "dims": 3,
                "usage": { "prompt_tokens": 3, "total_tokens": 3 }
            }),
            "one vector per text, in the order given"
        );
    }
}
