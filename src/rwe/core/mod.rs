pub mod compiler;
pub mod config;
pub mod deno_worker;
pub mod error;
pub mod js_masker;
pub mod model;
pub mod render;
pub mod security;

pub use config::{CompileOptions, RuntimeMode, SecurityPolicy};
pub use error::EngineError;
pub use model::{CompiledTemplate, RenderOutput};

pub fn compile(source: &str, options: CompileOptions) -> Result<CompiledTemplate, EngineError> {
    compiler::compile(source, options)
}

pub fn render(
    compiled: &CompiledTemplate,
    vars: &serde_json::Value,
) -> Result<RenderOutput, EngineError> {
    render::render(compiled, vars)
}

pub fn prewarm(compiled: &CompiledTemplate) -> Result<(), EngineError> {
    render::prewarm(compiled)
}

/// Prepare a template root directory for use with the RWE runtime.
///
/// This function must be called once after the template files have been
/// written to `root`. It:
///
/// 1. Writes the `rwe.ts` shim into the root so component files can
///    `import { useState, useNavigate, Link } from "rwe"`.
/// 2. Walks every `.tsx / .ts / .jsx / .js` file under `root` and
///    rewrites:
///    - `from "rwe"` / `from 'rwe'`   → absolute path of the shim
///    - `from "@/..."` / `from '@/...'` → absolute resolved file path
///
/// Both rewrites are idempotent: if a file already contains the
/// resolved absolute path the content comparison skips the write.
pub fn prepare_template_root(root: &std::path::Path) -> Result<(), EngineError> {
    use std::fs;
    use std::path::PathBuf;

    const RWE_SHIM: &str = include_str!("../runtime/rwe.ts");

    // --- 1. Write the shim -------------------------------------------------
    let shim_path = root.join("rwe.ts");
    fs::write(&shim_path, RWE_SHIM).map_err(|e| {
        EngineError::new("RWE_PREPARE_SHIM", format!("failed writing rwe shim: {e}"))
    })?;

    // --- 2. Collect all script files ----------------------------------------
    fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect(&path, out)?;
            } else if let Some(ext) = path.extension().and_then(|v| v.to_str()) {
                if matches!(ext, "tsx" | "ts" | "jsx" | "js") {
                    out.push(path);
                }
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, &mut files).map_err(|e| {
        EngineError::new("RWE_PREPARE_COLLECT", format!("failed collecting template files: {e}"))
    })?;

    // --- 3. Rewrite imports in each file ------------------------------------
    for file in files {
        if file == shim_path {
            continue; // skip the shim itself
        }
        let source = fs::read_to_string(&file).map_err(|e| {
            EngineError::new(
                "RWE_PREPARE_READ",
                format!("failed reading '{}': {e}", file.display()),
            )
        })?;
        let rewritten = rewrite_source(&source, root)?;
        if rewritten != source {
            fs::write(&file, rewritten).map_err(|e| {
                EngineError::new(
                    "RWE_PREPARE_WRITE",
                    format!("failed writing '{}': {e}", file.display()),
                )
            })?;
        }
    }

    Ok(())
}

fn rewrite_source(source: &str, root: &std::path::Path) -> Result<String, EngineError> {
    let mut out = source.to_string();
    out = rewrite_imports_variant(&out, root, '"')?;
    out = rewrite_imports_variant(&out, root, '\'')?;
    Ok(out)
}

fn rewrite_imports_variant(
    source: &str,
    root: &std::path::Path,
    quote: char,
) -> Result<String, EngineError> {
    use std::fs;

    let marker = format!("from {quote}");
    let mut out = source.to_string();
    let mut cursor = 0usize;

    while let Some(rel_idx) = out[cursor..].find(marker.as_str()) {
        let idx = cursor + rel_idx;
        let spec_start = idx + marker.len();
        let Some(end_rel) = out[spec_start..].find(quote) else {
            break;
        };
        let spec_end = spec_start + end_rel;
        let rel = &out[spec_start..spec_end];

        if rel == "rwe" {
            let shim = root.join("rwe.ts");
            let resolved = fs::canonicalize(&shim).unwrap_or(shim);
            let resolved_str = resolved.to_string_lossy().to_string();
            out.replace_range(spec_start..spec_end, &resolved_str);
            cursor = spec_start + resolved_str.len();
            continue;
        }

        if !rel.starts_with("@/") {
            cursor = spec_end + 1;
            continue;
        }

        let resolved = resolve_alias(root, rel.trim_start_matches("@/"))?;
        out.replace_range(spec_start..spec_end, &resolved);
        cursor = spec_start + resolved.len();
    }

    Ok(out)
}

fn resolve_alias(root: &std::path::Path, rel: &str) -> Result<String, EngineError> {
    use std::fs;
    use std::path::PathBuf;

    let base = root.join(rel);
    if base.exists() {
        let abs = fs::canonicalize(&base).unwrap_or(base);
        return Ok(abs.to_string_lossy().to_string());
    }
    for ext in [".tsx", ".ts", ".jsx", ".js"] {
        let candidate = PathBuf::from(format!("{}{ext}", base.display()));
        if candidate.exists() {
            let abs = fs::canonicalize(&candidate).unwrap_or(candidate);
            return Ok(abs.to_string_lossy().to_string());
        }
    }
    for index in ["index.tsx", "index.ts", "index.jsx", "index.js"] {
        let candidate = base.join(index);
        if candidate.exists() {
            let abs = fs::canonicalize(&candidate).unwrap_or(candidate);
            return Ok(abs.to_string_lossy().to_string());
        }
    }
    Ok(base.to_string_lossy().to_string())
}
