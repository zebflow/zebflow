//! Concrete framework engine implementations.

pub mod basic;
mod noop;

pub use basic::BasicFrameworkEngine;
pub use noop::NoopFrameworkEngine;
