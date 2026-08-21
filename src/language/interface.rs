use serde_json::Value;

use super::model::{
    CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput, LanguageError,
    ModuleSource, ProgramIr,
};

/// Engine-agnostic language contract used by Zebflow runtime.
pub trait LanguageEngine: Send + Sync {
    /// Stable engine id.
    fn id(&self) -> &'static str;

    /// Whether a script this engine runs may open an outbound connection.
    ///
    /// The answer must come from the engine's *effective* policy rather than
    /// from what it ships denying, because the caller asking is confining a
    /// node bundle and needs to know what an operator has granted here and now.
    ///
    /// It defaults to `true` so an engine that does not answer is assumed to
    /// reach the network, and is refused inside a bundle rather than quietly
    /// becoming the way out of that bundle's declared hosts.
    fn grants_network(&self) -> bool {
        true
    }

    /// Parses a source module into engine-specific intermediate representation.
    fn parse(&self, module: &ModuleSource) -> Result<ProgramIr, LanguageError>;

    /// Compiles IR into executable engine artifact.
    fn compile(
        &self,
        ir: &ProgramIr,
        options: &CompileOptions,
    ) -> Result<CompiledProgram, LanguageError>;

    /// Executes previously compiled artifact with JSON input.
    fn run(
        &self,
        compiled: &CompiledProgram,
        input: Value,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionOutput, LanguageError>;
}
