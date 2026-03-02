//! Zebflow core crate.
//!
//! This crate is intentionally split into independent subsystems:
//!
//! 1. [`framework`] for pipeline orchestration
//! 2. [`language`] for sandboxed script execution
//! 3. [`rwe`] for reactive web template compile/render
//! 4. [`automaton`] for autonomous objective planning/execution + interactive REPL (Zebtune)
//! 5. [`platform`] for service composition and web shell
//!
//! The [`ZebflowEngineKit`] type wires default implementations so an app entrypoint
//! can keep `main.rs` thin and delegate all behavior to library modules.

pub mod automaton;
pub mod framework;
pub mod language;
pub mod platform;
pub mod rwe;

use std::sync::Arc;

use automaton::{AutomatonEngine, AutomatonEngineRegistry, NoopAutomatonEngine};
use framework::{
    BasicFrameworkEngine, FrameworkEngine, FrameworkEngineRegistry, NoopFrameworkEngine,
};
use language::{DenoSandboxEngine, LanguageEngine, LanguageEngineRegistry, NoopLanguageEngine};
use rwe::{NoopReactiveWebEngine, ReactiveWebEngine, ReactiveWebEngineRegistry};

/// Ready-to-use set of engine registries for framework/language/rwe modules.
///
/// This is the main composition root used by hosts (CLI, server, tests) to
/// lookup engine implementations by id.
#[derive(Clone)]
pub struct ZebflowEngineKit {
    /// Automaton engines.
    pub automaton: AutomatonEngineRegistry,
    /// Pipeline orchestration engines.
    pub framework: FrameworkEngineRegistry,
    /// Script/runtime engines.
    pub language: LanguageEngineRegistry,
    /// Reactive web engines.
    pub rwe: ReactiveWebEngineRegistry,
}

impl ZebflowEngineKit {
    /// Builds a kit with default engines registered:
    ///
    /// - `framework.basic`
    /// - `framework.noop`
    /// - `language.deno_sandbox`
    /// - `language.noop`
    /// - `rwe.noop`
    /// - `automaton.noop`
    pub fn with_defaults() -> Self {
        let mut automaton = AutomatonEngineRegistry::new();
        automaton.register(Arc::new(NoopAutomatonEngine));

        let mut framework = FrameworkEngineRegistry::new();
        framework.register(Arc::new(BasicFrameworkEngine::default()));
        framework.register(Arc::new(NoopFrameworkEngine::default()));

        let mut language = LanguageEngineRegistry::new();
        language.register(Arc::new(DenoSandboxEngine::default()));
        language.register(Arc::new(NoopLanguageEngine::default()));

        let mut rwe = ReactiveWebEngineRegistry::new();
        rwe.register(Arc::new(NoopReactiveWebEngine::default()));

        Self {
            automaton,
            framework,
            language,
            rwe,
        }
    }

    /// Returns an automaton engine by id.
    pub fn automaton_engine(&self, id: &str) -> Option<Arc<dyn AutomatonEngine>> {
        self.automaton.get(id)
    }

    /// Returns a framework engine by id.
    pub fn framework_engine(&self, id: &str) -> Option<Arc<dyn FrameworkEngine>> {
        self.framework.get(id)
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
