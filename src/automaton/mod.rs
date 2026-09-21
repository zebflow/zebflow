//! # Automaton
//!
//! What `n.ai.agent` and the Studio assistant run on. Five pieces, nothing
//! else:
//!
//! - [`infra::llm_interface`] — the one LLM call interface: `LlmCall`
//!   (`call`, `call_with_tools`), `ToolDef`, `ToolCall`, `CallResult`,
//!   `Usage`, `Message`, `MessageRole`.
//! - [`infra::http_client`] — one OpenAI-compatible client,
//!   `OpenAiHttpClient`, with two surfaces (Responses for the `openai`
//!   credential kind, Chat Completions for `openrouter`), and the
//!   credential-driven factories `client_from_provider_secret_with_model`,
//!   `client_from_secret`, `client_from_secret_with_model`.
//! - [`agents::zebtune`] — the one model loop, `ZebtuneAgent`: send the
//!   messages with the tool definitions, run each tool call the model
//!   returns, append the results, call again, stop when the model answers
//!   with text or the step budget is spent; an optional JSON-Schema
//!   contract and verifier gate the answer, and a failure is fed back for
//!   repair.
//! - [`agents::contract`] — the JSON-Schema subset check (`check_contract`,
//!   `Verdict`) and the tolerant JSON extractor (`extract_json`).
//! - [`infra::assistant_config`] — `load_project_assistant_llm`, the Studio
//!   assistant's LLM settings loader (Settings > Automatons).
//!
//! Independent of the platform and pipeline modules: nothing here knows
//! what a pipeline or a project is beyond the credential secret it is
//! handed. Anthropic is not a provider yet; its tool-use mapping is a
//! separate decision (see `http_client.rs`).
//!
//! ## Security model
//!
//! A tool is offered only when the host names it (for `n.ai.agent`: one of the
//! project's function pipelines, by slug). There are no shell tools. The step
//! budget is a hard cap on model calls per run. The only source of an LLM
//! secret is a project credential; there is no environment fallback.

pub mod agents;
pub mod infra;

pub use agents::zebtune::ZebtuneAgent;
