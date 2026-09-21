//! Zebflow core crate.
//!
//! This crate is intentionally split into independent subsystems:
//!
//! 1. [`contracts`] for stable persisted and transferred format boundaries
//! 2. [`pipeline`] for pipeline orchestration (graph traversal, node dispatch)
//! 3. [`language`] for sandboxed script execution (Deno)
//! 4. [`rwe`] for reactive web template compile/render (TSX → SSR → hydrate)
//! 5. [`automaton`] for the LLM call interface, the OpenAI-compatible client and the model loop `n.ai.agent` runs
//! 6. [`platform`] for service composition and web shell (Axum, MCP, DSL)
//! 7. [`infra`] for shared runtime infrastructure (WebSocket, storage, scheduler)
//!
//! The [`ZebflowEngineKit`] type wires default implementations so an app entrypoint
//! can keep `main.rs` thin and delegate all behavior to library modules.

pub mod automaton;
pub mod contracts;
pub mod infra;
pub mod language;
pub mod mapserver;
pub mod pipeline;
pub mod platform;
pub mod provision;
pub mod rwe;
pub mod version;
pub mod zebfs;

use std::sync::Arc;

use language::{DenoSandboxEngine, LanguageEngine, LanguageEngineRegistry, NoopLanguageEngine};
use pipeline::{BasicPipelineEngine, NoopPipelineEngine, PipelineEngine, PipelineEngineRegistry};
use rwe::{ReactiveWebEngine, ReactiveWebEngineRegistry, RweReactiveWebEngine};

/// Ready-to-use set of engine registries for pipeline/language/rwe modules.
///
/// This is the main composition root used by hosts (CLI, server, tests) to
/// lookup engine implementations by id.
#[derive(Clone)]
pub struct ZebflowEngineKit {
    /// Pipeline execution engines.
    pub pipeline: PipelineEngineRegistry,
    /// Script/runtime engines.
    pub language: LanguageEngineRegistry,
    /// Reactive web engines.
    pub rwe: ReactiveWebEngineRegistry,
}

impl ZebflowEngineKit {
    /// Builds a kit with default engines registered:
    ///
    /// - `pipeline.basic`
    /// - `pipeline.noop`
    /// - `language.deno_sandbox`
    /// - `language.noop`
    /// - `rwe`
    pub fn with_defaults() -> Self {
        let mut pipeline = PipelineEngineRegistry::new();
        pipeline.register(Arc::new(BasicPipelineEngine::default()));
        pipeline.register(Arc::new(NoopPipelineEngine::default()));

        let mut language = LanguageEngineRegistry::new();
        language.register(Arc::new(DenoSandboxEngine::default()));
        language.register(Arc::new(NoopLanguageEngine::default()));

        let mut rwe = ReactiveWebEngineRegistry::new();
        rwe.register(Arc::new(RweReactiveWebEngine::default()));

        Self {
            pipeline,
            language,
            rwe,
        }
    }

    /// Returns a pipeline execution engine by id.
    pub fn pipeline_engine(&self, id: &str) -> Option<Arc<dyn PipelineEngine>> {
        self.pipeline.get(id)
    }

    /// Returns a language engine by id.
    pub fn language_engine(&self, id: &str) -> Option<Arc<dyn LanguageEngine>> {
        self.language.get(id)
    }

    /// Returns an RWE engine by id.
    pub fn rwe_engine(&self, id: &str) -> Option<Arc<dyn ReactiveWebEngine>> {
        self.rwe.get(id)
    }
}
