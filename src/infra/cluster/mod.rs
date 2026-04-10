//! Cluster topology, security, and control-plane transport.
//!
//! This layer is the future home of master/worker runtime coordination.
//! It should stay focused on:
//!
//! - role configuration
//! - secure join/bootstrap
//! - node certificates and trust
//! - control-plane transport
//! - worker registry and heartbeat
//! - Kubernetes bootstrap/render helpers
//!
//! It should *not* absorb business logic from `platform/services/`.

pub mod config;
pub mod k8s;
pub mod registry;
pub mod security;
pub mod transport;
