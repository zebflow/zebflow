//! Platform policy checks.
//!
//! Policy is platform-owned decision logic: it inspects packages, files, and
//! runtime capabilities and returns structured facts for UI/API callers to
//! render. It should not depend on React templates, request handlers, or other
//! presentation details.

pub mod package;
pub mod report;
