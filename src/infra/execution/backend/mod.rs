//! Execution backend catalog.
//!
//! A backend describes *how* a whole pipeline run is executed.
//! The first real backends are expected to be:
//!
//! - resident local execution
//! - resident remote worker execution
//!
//! Future backends include isolated job-oriented execution such as Docker, Kubernetes,
//! or Spark-oriented submission backends.

pub mod docker_job;
pub mod interface;
pub mod k8s_job;
pub mod resident_local;
pub mod resident_worker;
pub mod spark_submit;

pub use interface::{
    DynExecutionBackend, ExecutionBackend, ExecutionBackendKind, ExecutionError,
    ExecutionProfileKind, ExecutionRequest,
};
pub use resident_local::ResidentLocalBackend;
pub use resident_worker::ResidentWorkerBackend;
