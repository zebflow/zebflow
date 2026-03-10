//! Zebflow core crate.
//!
//! This crate is intentionally split into independent subsystems:
//!
//! 1. [`pipeline`] for pipeline orchestration
//! 2. [`language`] for sandboxed script execution
//! 3. [`rwe`] for reactive web template compile/render
//! 4. [`automaton`] for autonomous objective planning/execution + interactive REPL (Zebtune)
//! 5. [`platform`] for service composition and web shell
//!
//! The [`ZebflowEngineKit`] type wires default implementations so an app entrypoint
//! can keep `main.rs` thin and delegate all behavior to library modules.

pub mod automaton;
pub mod pipeline;
pub mod language;
pub mod llm;
pub mod platform;
pub mod rwe;

use std::sync::Arc;

use automaton::{AutomatonEngine, AutomatonEngineRegistry, NoopAutomatonEngine};
use pipeline::{
    BasicFrameworkEngine, FrameworkEngine, FrameworkEngineRegistry, NoopFrameworkEngine,
};
use language::{DenoSandboxEngine, LanguageEngine, LanguageEngineRegistry, NoopLanguageEngine};
use rwe::{ReactiveWebEngine, ReactiveWebEngineRegistry, RweReactiveWebEngine};

/// Ready-to-use set of engine registries for pipeline/language/rwe modules.
///
/// This is the main composition root used by hosts (CLI, server, tests) to
/// lookup engine implementations by id.
#[derive(Clone)]
pub struct ZebflowEngineKit {
    /// Automaton engines.
    pub automaton: AutomatonEngineRegistry,
    /// Pipeline execution engines.
    pub pipeline: FrameworkEngineRegistry,
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
    /// - `automaton.noop`
    pub fn with_defaults() -> Self {
        let mut automaton = AutomatonEngineRegistry::new();
        automaton.register(Arc::new(NoopAutomatonEngine));

        let mut pipeline = FrameworkEngineRegistry::new();
        pipeline.register(Arc::new(BasicFrameworkEngine::default()));
        pipeline.register(Arc::new(NoopFrameworkEngine::default()));

        let mut language = LanguageEngineRegistry::new();
        language.register(Arc::new(DenoSandboxEngine::default()));
        language.register(Arc::new(NoopLanguageEngine::default()));

        let mut rwe = ReactiveWebEngineRegistry::new();
        rwe.register(Arc::new(RweReactiveWebEngine::default()));

        Self {
            automaton,
            pipeline,
            language,
            rwe,
        }
    }

    /// Returns an automaton engine by id.
    pub fn automaton_engine(&self, id: &str) -> Option<Arc<dyn AutomatonEngine>> {
        self.automaton.get(id)
    }

    /// Returns a pipeline execution engine by id.
    pub fn pipeline_engine(&self, id: &str) -> Option<Arc<dyn FrameworkEngine>> {
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
