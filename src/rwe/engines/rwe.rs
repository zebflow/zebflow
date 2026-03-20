use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::language::LanguageEngine;
use crate::rwe::interface::ReactiveWebEngine;
use crate::rwe::model::{
    CompiledScript, CompiledScriptScope, CompiledTemplate, ReactiveBinding, ReactiveMode,
    ReactiveWebDiagnostic, ReactiveWebError, ReactiveWebOptions, RenderContext, RenderOutput,
    RuntimeBundle, RuntimeMode, TemplateSource,
};
use crate::rwe::processors;

#[derive(Default)]
pub struct RweReactiveWebEngine;

impl ReactiveWebEngine for RweReactiveWebEngine {
    fn id(&self) -> &'static str {
        "rwe"
    }

    fn compile_template(
        &self,
        template: &TemplateSource,
        _language: &dyn LanguageEngine,
        options: &ReactiveWebOptions,
    ) -> Result<CompiledTemplate, ReactiveWebError> {
        let compile_options = crate::rwe::core::CompileOptions {
            template_root: options
                .templates
                .template_root
                .as_ref()
                .map(|p| p.display().to_string()),
            file_path: template
                .source_path
                .as_ref()
                .map(|p| p.display().to_string()),
            runtime_mode: match options.runtime_mode {
                RuntimeMode::Dev | RuntimeMode::Prod => crate::rwe::core::RuntimeMode::Inline,
            },
            security: crate::rwe::core::SecurityPolicy {
                import_allowlist: {
                    let mut allow = vec!["@/".to_string()];
                    allow.extend(options.allow_list.urls.clone());
                    allow
                },
                network_allowlist: if options.allow_list.urls.is_empty() {
                    vec!["registry.npmjs.org".to_string(), "jsr.io".to_string()]
                } else {
                    options.allow_list.urls.clone()
                },
                blocked_globals: vec![
                    "eval".to_string(),
                    "Function".to_string(),
                    "globalThis.Function".to_string(),
                ],
                allow_dynamic_import: false,
                allow_raw_html: false,
            },
            deno_timeout_ms: 3_000,
        };

        let compiled = crate::rwe::core::compile(&template.markup, compile_options).map_err(|err| {
            ReactiveWebError::new(
                "RWE_COMPILE",
                format!("rwe compile failed for '{}': {}", template.id, err.message),
            )
        })?;

        // Best-effort warmup so post-save first request does not pay cold Deno path.
        let warmup_enabled = std::env::var("ZEBFLOW_RWE_PREWARM")
            .map(|v| v != "0")
            .unwrap_or(true);
        if warmup_enabled {
            let warm_compiled = compiled.clone();
            std::thread::spawn(move || {
                let _ = crate::rwe::core::prewarm(&warm_compiled);
            });
        }

        let payload = serde_json::to_value(&compiled).map_err(|err| {
            ReactiveWebError::new(
                "RWE_PAYLOAD",
                format!("failed serializing rwe payload: {err}"),
            )
        })?;

        let diagnostics = compiled
            .diagnostics
            .iter()
            .map(|d| ReactiveWebDiagnostic {
                code: d.code.clone(),
                message: d.message.clone(),
            })
            .collect::<Vec<_>>();

        Ok(CompiledTemplate {
            engine_id: self.id().to_string(),
            template_id: template.id.clone(),
            html_ir: "<!-- rwe html plan -->".to_string(),
            control_script_source: None,
            compiled_logic: None,
            runtime_bundle: RuntimeBundle {
                name: "rwe-runtime".to_string(),
                source: String::new(),
            },
            reactive_bindings: if options.reactive_mode == ReactiveMode::Bindings {
                vec![ReactiveBinding {
                    kind: "rwe".to_string(),
                    key: "event-plan".to_string(),
                }]
            } else {
                Vec::new()
            },
            diagnostics,
            needs_runtime_tailwind_rebuild: false,
            tailwind_variant_exact_tokens: Vec::new(),
            tailwind_variant_patterns: Vec::new(),
            options: options.clone(),
            engine_payload: Some(payload),
        })
    }

    fn render(
        &self,
        compiled: &CompiledTemplate,
        state: Value,
        _language: &dyn LanguageEngine,
        _ctx: &RenderContext,
    ) -> Result<RenderOutput, ReactiveWebError> {
        let payload = compiled.engine_payload.as_ref().ok_or_else(|| {
            ReactiveWebError::new(
                "RWE_PAYLOAD_MISSING",
                "compiled template missing rwe engine payload",
            )
        })?;

        let rwe_compiled: crate::rwe::core::CompiledTemplate =
            serde_json::from_value(payload.clone()).map_err(|err| {
                ReactiveWebError::new(
                    "RWE_PAYLOAD_PARSE",
                    format!("failed parsing rwe payload: {err}"),
                )
            })?;

        let rendered = crate::rwe::core::render(&rwe_compiled, &state).map_err(|err| {
            ReactiveWebError::new(
                "RWE_RENDER",
                format!("rwe render failed: {}", err.message),
            )
        })?;

        // Tailwind stays on Zebflow processor pipeline.
        let mut processor_diags: Vec<ReactiveWebDiagnostic> = Vec::new();
        let processed_html =
            processors::apply_compile_processors(&rendered.html, &compiled.options, &mut processor_diags);
        let (clean_html, extracted_css) = extract_generated_tailwind_style(&processed_html);

        let mut scripts = Vec::new();
        if !rendered.js.is_empty() {
            let hash = stable_hash_hex(&rendered.js);
            let script_id = format!("rwe.page.{}", compiled.template_id);
            scripts.push(CompiledScript {
                id: script_id,
                scope: CompiledScriptScope::Page,
                content_type: "text/javascript".to_string(),
                content: rendered.js,
                content_hash: hash.clone(),
                suggested_file_name: format!("rwe-{hash}.mjs"),
            });
        }

        Ok(RenderOutput {
            html: clean_html,
            compiled_scripts: scripts,
            hydration_payload: json!({
                "engine": "rwe",
                "css": extracted_css,
                "meta": rendered.meta,
                "processor_diagnostics": processor_diags,
            }),
            trace: vec!["rwe.render".to_string()],
        })
    }
}

fn stable_hash_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn extract_generated_tailwind_style(html: &str) -> (String, String) {
    const OPEN: &str = "<style data-rwe-tw>";
    const CLOSE: &str = "</style>";

    if let Some(start) = html.find(OPEN) {
        let content_start = start + OPEN.len();
        if let Some(end_rel) = html[content_start..].find(CLOSE) {
            let end = content_start + end_rel;
            let css = html[content_start..end].to_string();
            let mut out = String::with_capacity(html.len());
            out.push_str(&html[..start]);
            out.push_str(&html[end + CLOSE.len()..]);
            return (out, css);
        }
    }

    (html.to_string(), String::new())
}
