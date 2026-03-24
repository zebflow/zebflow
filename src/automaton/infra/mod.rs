//! Automaton infrastructure — plumbing only, not agents, not capabilities.
//!
//! Contains LLM clients, HTTP adapters, REPL utilities, and shell execution tools.
//! Everything the agents layer BUILDS ON TOP OF, but nothing that constitutes intelligence itself.

pub mod assistant_config;
pub mod http_client;
pub mod interface;
pub mod llm;
pub mod llm_interface;
pub mod model;
pub mod registry;
pub mod repl;
pub mod shell_tools;
