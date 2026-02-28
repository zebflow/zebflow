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
use markdown::process_markdown;
use tailwind::process_tailwind;

pub mod markdown;
pub mod tailwind;

/// Apply enabled compile processors.
///
/// Resolution rules:
///
/// - if `options.processors` is empty, use legacy behavior (Tailwind-like
///   processing follows `options.style_engine`)
/// - if `options.processors` is non-empty, only listed processors run, in the
///   listed order
pub fn apply_compile_processors(
    html: &str,
    options: &ReactiveWebOptions,
    diagnostics: &mut Vec<ReactiveWebDiagnostic>,
) -> String {
    let normalized = normalize_processor_list(&options.processors);
    if normalized.is_empty() {
        return match options.style_engine {
            StyleEngineMode::TailwindLike => process_tailwind(html),
            StyleEngineMode::Off => html.to_string(),
        };
    }

    let mut out = html.to_string();
    for processor in normalized {
        match processor.as_str() {
            "tailwind" => {
                out = process_tailwind(&out);
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
