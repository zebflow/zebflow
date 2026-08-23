//! Cluster/node-role configuration model.
//!
//! This module defines the minimum process-level settings required to boot Zebflow in different
//! topologies without changing the product surface:
//!
//! - standalone: today's all-in-one local install
//! - controller: control-plane process
//! - office: execution-plane process
//!
//! The point of this module is to keep role intent explicit and serializable early, before the
//! transport and security layers are fully wired.

pub mod role;
pub mod settings;

pub use role::{ClusterRole, ServerMode};
pub use settings::{ClusterConfigError, ClusterSettings};
