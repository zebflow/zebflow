//! The model loop and its contract check.
//!
//! - [`zebtune::ZebtuneAgent`] — the one loop `n.ai.agent` runs: send the
//!   messages with the tool definitions, run each tool call the model
//!   returns, append the results, call again, stop when the model answers
//!   with text or the budget is spent. An optional contract — a JSON schema
//!   and/or a verifier closure — gates the answer; a failure is fed back for
//!   repair. Emits a `ChainStep` per event so a host can stream progress.
//!   With no tools it is one call.
//! - [`contract`] — the JSON Schema subset check (type, required,
//!   properties, items, enum, minItems, minLength) and the tolerant JSON
//!   extractor the loop uses.

pub mod contract;
pub mod zebtune;

pub use zebtune::ZebtuneAgent;
