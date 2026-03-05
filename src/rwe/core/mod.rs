pub mod compiler;
pub mod config;
pub mod deno_worker;
pub mod error;
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
