use serde_json::Value;

use crate::language::LanguageEngine;

use super::model::{
    CompiledTemplate, ReactiveWebError, ReactiveWebOptions, RenderContext, RenderOutput,
    TemplateSource,
};

/// Engine-agnostic interface for TSX compile/render backends.
pub trait ReactiveWebEngine: Send + Sync {
    /// Stable engine id.
    fn id(&self) -> &'static str;

    /// Compiles template source into an engine-specific artifact.
    fn compile_template(
        &self,
        template: &TemplateSource,
        language: &dyn LanguageEngine,
        options: &ReactiveWebOptions,
    ) -> Result<CompiledTemplate, ReactiveWebError>;

    /// Renders a compiled template with request state/context.
    fn render(
        &self,
        compiled: &CompiledTemplate,
        state: Value,
        language: &dyn LanguageEngine,
        ctx: &RenderContext,
    ) -> Result<RenderOutput, ReactiveWebError>;
}
