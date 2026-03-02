//! MCP (Model Context Protocol) server for Zebflow platform.
//!
//! Exposes project-scoped management tools to LLM-based agents (Cursor, etc.)
//! via per-project session tokens.

mod handler;

pub use handler::build_mcp_service;
