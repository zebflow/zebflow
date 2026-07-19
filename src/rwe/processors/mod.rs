//! Compile-stage processor pipeline for RWE templates.
//!
//! Processors are optional features that transform template HTML before final
//! compile artifacts are produced.
//!
//! Current processors:
//!
//! - `tailwind`: Tailwind-like utility token compiler
//! - `markdown`: Markdown block conversion (`<markdown>...</markdown>`)

use crate::rwe::model::{ReactiveWebDiagnostic, ReactiveWebOptions, StyleEngineMode};
use markdown::{process_markdown, process_rwe_md_placeholders};
use tailwind::{collect_source_tokens, process_tailwind};

pub mod markdown;
pub mod tailwind;

/// Apply enabled compile processors.
///
/// Resolution rules:
///
/// - if `options.processors` is empty, default behavior applies: Tailwind-like
///   processing is driven by `options.style_engine`
/// - if `options.processors` is non-empty, only listed processors run, in the
///   listed order
pub fn apply_compile_processors(
    html: &str,
    source: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> String {
    let normalized = normalize_processor_list(&options.processors);
    let extra = collect_source_tokens(source);
    let mut out = if normalized.is_empty() {
        match options.style_engine {
            StyleEngineMode::TailwindLike => process_tailwind(html, &extra),
            StyleEngineMode::Off => html.to_string(),
        }
    } else {
        let mut out = html.to_string();
        for processor in normalized {
            match processor.as_str() {
                "tailwind" => {
                    out = process_tailwind(&out, &extra);
                }
                "markdown" => {
                    out = process_markdown(&out);
                }
                other => diagnostics.push(ReactiveWebDiagnostic {
                    code: "RWE_PROCESSOR_UNKNOWN".to_string(),
                    message: format!("unknown processor '{other}' (ignored)"),
                }),
            }
        }
        out
    };
    // Always process <Markdown> component placeholders (data-rwe-md attributes)
    // regardless of explicit processor list — this is a core RWE feature.
    out = process_rwe_md_placeholders(&out);
    out
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    use super::*;

    #[test]
    fn default_tailwind_mode_still_renders_markdown_component_placeholders() {
        let encoded = BASE64.encode("## Mapped story\n\nA spatial field note.");
        let html = format!(
            "<div class=\"rwe-md-placeholder prose\" data-rwe-md=\"{encoded}\" data-zeb-lib=\"markdown\"></div>"
        );
        let options = ReactiveWebOptions {
            style_engine: StyleEngineMode::TailwindLike,
            ..Default::default()
        };
        let mut diagnostics = Vec::new();

        let output = apply_compile_processors(&html, "", &options, &mut diagnostics);

        assert!(output.contains("<h2 id=\"mapped-story\">Mapped story</h2>"));
        assert!(output.contains("<p>A spatial field note.</p>"));
        assert!(!output.contains("data-rwe-md"));
        assert!(diagnostics.is_empty());
    }
}

fn normalize_processor_list(raw: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for item in raw {
        let lowered = item.trim().to_ascii_lowercase();
        if lowered.is_empty() {
            continue;
        }
        if out.iter().any(|existing| existing == &lowered) {
            continue;
        }
        out.push(lowered);
    }
    out
}
