//! Automaton memory subsystem.
//!
//! Memory is a first-class concern in Zebflow's automaton architecture.
//! Different memory strategies have meaningfully different trade-offs in how
//! an autonomous agent retains and retrieves context.
//!
//! ## Current
//!
//! - [`basic`] — conversation history with auto-compression + token tracking.
//!   Simple sliding-window approach; sufficient for most single-session tasks.
//!
//! ## Planned
//!
//! - `graph/` — graph-structured memory: entities as nodes, relations as edges.
//!   Enables multi-hop reasoning over accumulated knowledge.
//! - `vector/` — embedding-based retrieval: store chunks, recall by semantic similarity.
//!   Best for large knowledge bases or document-grounded agents.
//! - `episodic/` — episodic memory: recall past runs/sessions by similarity or recency.
//! - `semantic/` — semantic memory: persistent factual knowledge, independent of conversation.

pub mod basic;

pub use basic::{ConversationHistory, SimpleTokenCounter, TokenCounter, TokenUsage};
