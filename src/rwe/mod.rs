//! Reactive Web Engine (RWE) module.
//!
//! Responsibility:
//!
//! - compile `.tsx` templates
//! - render HTML + hydration payload
//! - optionally integrate language engines for control scripts

pub mod axum_demo;
pub(crate) mod class_notation;
pub(crate) mod core;
pub mod engines;
pub mod interface;
pub mod model;
pub mod processors;
pub mod protocol;
pub mod registry;
pub mod script_cache;

pub use engines::{
    RweReactiveWebEngine, instantiate_engine_by_id, resolve_engine_or_default,
};
pub use interface::ReactiveWebEngine;
pub use model::{
    CompiledScript, CompiledScriptScope, CompiledTemplate, ComponentOptions, LanguageOptions,
    ReactiveBinding, ReactiveMode, ReactiveWebDiagnostic, ReactiveWebError, ReactiveWebOptions,
    RenderContext, RenderOutput, ResourceAllowList, RuntimeBundle, RuntimeMode, StyleEngineMode,
    TemplateOptions, TemplateSource,
};
pub use protocol::{
    CompileTemplateRequest, CompileTemplateResponse, ProtocolError, ProtocolMeta,
    RWE_PROTOCOL_VERSION, RenderTemplateRequest, RenderTemplateResponse,
};
pub use registry::ReactiveWebEngineRegistry;
pub use script_cache::{CachedScriptRef, RenderScriptCache, ScriptCacheConfig};
