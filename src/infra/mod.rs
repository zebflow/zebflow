//! Infrastructure layer — shared runtime services and distributed-system seams.
//!
//! This module is where Zebflow's reusable runtime plumbing lives. The rule is:
//! business features belong in `pipeline/`, `rwe/`, `language/`, or `platform/`;
//! reusable mechanics belong in `infra/`.
//!
//! # High-level map
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`io`] | Swappable stores, buses, and cache contracts |
//! | [`execution`] | Whole-pipeline execution backends, placement, and runner metadata |
//! | [`cluster`] | Future master/worker topology, secure control transport, and K8s hooks |
//! | [`mem`] | Current in-process KV/pubsub implementation used as the first `StateBus` backend (`n.kv.*` nodes) |
//! | [`transport`] | Real-time transport primitives, currently WebSocket rooms |
//! | [`scheduler`] | Background cron scheduling for active pipelines |
//! | [`storage`] | Older storage placeholder area; retained until the new `io/` split fully absorbs it |
//!
//! # Design principles
//!
//! 1. **Standalone remains first-class**
//!    The same binary must still run as a tiny local install or on a Raspberry Pi.
//! 2. **Topology is separate from business logic**
//!    `cluster/` decides where work runs, not what a pipeline does.
//! 3. **Execution is separate from integration**
//!    Talking to Spark/HDFS/Kafka is a node concern. Routing an entire pipeline run to
//!    a resident worker, Kubernetes job, or future Spark submit backend is an execution concern.
//! 4. **Swappable backends live behind explicit interfaces**
//!    The goal is to make `mem` → Redis, local FS → object storage, and local execution →
//!    remote execution a wiring change rather than a product rewrite.
//!
//! # What belongs in `infra/`
//!
//! A thing belongs in `infra/` when it:
//!
//! - changes the runtime shape of the whole application
//! - is shared by both pipeline execution and platform services
//! - exists to route, store, transport, cache, or coordinate work
//! - needs to remain replaceable independently of product features

pub mod cluster;
pub mod execution;
pub mod health;
pub mod io;
pub mod mem;
pub mod scheduler;
pub mod storage;
pub mod transport;
pub mod ws_client;
