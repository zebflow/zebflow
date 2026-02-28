use serde_json::Value;

use super::model::{
    CompileOptions, CompiledProgram, ExecutionContext, ExecutionOutput, LanguageError,
    ModuleSource, ProgramIr,
};

/// Engine-agnostic language contract used by Zebflow runtime.
pub trait LanguageEngine: Send + Sync {
    /// Stable engine id.
    fn id(&self) -> &'static str;

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
