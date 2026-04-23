//! Lightweight spatial publish/resolve engine.
//!
//! First-principle split:
//!
//! - `resolve` — runtime brain for map-shaped requests
//! - `publish` — published layer manifests and registration surface
//! - `infra` — source adapters and shared support for both

pub mod infra;
pub mod publish;
pub mod resolve;
