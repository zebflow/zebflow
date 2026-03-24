//! LLM client factories.
//!
//! Re-exports `client_from_secret`, `client_from_secret_with_model`, and `client_from_env`
//! from `http_client`. All return `Arc<dyn LlmCall>` — the single LLM interface.
//!
//! This module exists for backward-compatible import paths:
//!   `zebflow::automaton::llm::client_from_env()`

pub use super::http_client::{client_from_env, client_from_secret, client_from_secret_with_model};
