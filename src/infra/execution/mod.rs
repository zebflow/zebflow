//! Whole-pipeline execution architecture.
//!
//! This layer answers questions such as:
//!
//! - should this pipeline run locally or on a remote worker?
//! - should it later run as a Kubernetes job or another specialized backend?
//! - which runner is capable of receiving it?
//! - how are logs, status, and cancellation represented?
//!
//! # Important boundary
//!
//! This is **not** the place for node-level integrations like Spark or Kafka clients.
//! Those belong in pipeline nodes.
//!
//! This layer is about routing and hosting an entire pipeline run.

pub mod backend;
pub mod handle;
pub mod placement;
pub mod runner;
pub mod sync;
