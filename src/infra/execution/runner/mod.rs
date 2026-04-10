//! Runner metadata model.
//!
//! A runner is a concrete execution target:
//!
//! - the standalone local runtime
//! - a long-lived worker runtime
//! - later a pool member or specialized compute target

pub mod capabilities;
pub mod descriptor;

pub use capabilities::RunnerCapabilities;
pub use descriptor::RunnerDescriptor;
