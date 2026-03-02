//! DB runtime domain (driver registry + per-kind describe/query execution).

pub mod driver;
pub mod drivers;
pub mod registry;

pub use driver::{DbDriver, DbDriverContext};
pub use registry::DbDriverRegistry;
