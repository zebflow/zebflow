//! Plumbing under the loop: the call interface, the one HTTP client, the
//! Studio assistant's settings loader.
//!
//! - [`llm_interface`] — `LlmCall` and the types on its wire (`ToolDef`,
//!   `ToolCall`, `CallResult`, `Usage`, `Message`, `MessageRole`).
//! - [`http_client`] — `OpenAiHttpClient` (Responses for `openai`, Chat
//!   Completions for `openrouter`) and the credential-driven factories.
//! - [`assistant_config`] — `load_project_assistant_llm` for the Studio
//!   assistant.

pub mod assistant_config;
pub mod http_client;
pub mod llm_interface;
