//! Cluster/node-role configuration model.
//!
//! This module defines the minimum process-level settings required to boot Zebflow in different
//! topologies without changing the product surface:
//!
//! - standalone: today's all-in-one local install
//! - master: future control-plane process
//! - worker: future execution-plane process
//!
//! The point of this module is to keep role intent explicit and serializable early, before the
//! transport and security layers are fully wired.

pub mod role;
pub mod settings;

pub use role::ClusterRole;
pub use settings::ClusterSettings;
