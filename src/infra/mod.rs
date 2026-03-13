//! Infrastructure layer — shared runtime services used by both pipeline nodes and platform.
//!
//! # Categories
//!
//! | Module | Responsibility |
//! |---|---|
//! | [`transport`] | Real-time connection management (WebSocket rooms, future: MQTT, SSE) |
//! | [`storage`] | Persistent data adapters (Sekejap, PostgreSQL) — stubs, WIP |
//! | [`scheduler`] | Background job scheduling (cron) — stubs, WIP |
//!
//! # Infra vs. Pipeline nodes
//!
//! A node file that only wraps an external API (S3, GCP, Apify) lives directly in
//! `pipeline/nodes/`. Something belongs in `infra/` when it either:
//!
//! 1. Needs to change platform structure (e.g. WS requires Axum routes, shared hub state).
//! 2. Is used at BOTH the pipeline node level AND the platform service level
//!    (e.g. a DB adapter used by `n.pg.query` nodes AND by `platform/services/`).

pub mod transport;
pub mod storage;
pub mod scheduler;
