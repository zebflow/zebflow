//! Swappable infrastructure engines: metadata, objects, runtime data, state bus, and cache.
//!
//! This layer exists so Zebflow can keep one product model while swapping implementations:
//!
//! - local filesystem vs. object storage
//! - in-memory bus vs. Redis-like shared bus
//! - process-local caches vs. distributed caches
//! - platform-local metadata vs. future externally managed catalog stores
//!
//! # Structure
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`catalog`] | Platform metadata contracts and catalog-backed implementations |
//! | [`object`] | Repo/files/assets/bundle storage |
//! | [`runtime_data`] | Per-project runtime data roots and engines |
//! | [`state`] | Shared KV/pubsub/lease semantics |
//! | [`cache`] | Generic cache contracts and implementations |
//!
//! # Boundary rule
//!
//! `io/` is about *where state lives* and *how it is exchanged*.
//! It is not where project/business decisions belong.

pub mod cache;
pub mod catalog;
pub mod object;
pub mod runtime_data;
pub mod state;
