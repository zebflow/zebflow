//! Runner boundary — dispatches compiled scripts to the embedded pool.
//!
//! The old subprocess implementation (`Command::new("deno")`) is replaced
//! entirely. No external binary, no temp files, no process management.

use serde_json::Value;

use super::engine::CompiledDenoSandboxScript;
use super::pool::{ScriptWork, run_in_pool};

/// Execute a compiled sandbox script using the embedded worker pool.
pub(crate) fn run_compiled_script(
    compiled: &CompiledDenoSandboxScript,
    input: &Value,
    ctx: Value,
) -> Result<Value, String> {
    run_in_pool(ScriptWork {
        fn_source: compiled.fn_source.clone(),
        config: compiled.resolved_config.clone(),
        input: input.clone(),
        ctx,
    })
}
