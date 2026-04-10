//! Lease-key helpers built on top of [`super::StateBus`].
//!
//! The first implementation only standardizes naming. Strong lease semantics are expected
//! to arrive when a shared backend such as Redis is introduced.

/// Logical lease scope used for key construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseScope {
    /// Lease attached to a whole project runtime.
    Project,
    /// Lease attached to one pipeline file.
    Pipeline,
    /// Lease attached to a scheduler or singleton task.
    Scheduler,
}

impl LeaseScope {
    fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Pipeline => "pipeline",
            Self::Scheduler => "scheduler",
        }
    }
}

/// Build a namespaced lease key suitable for a `StateBus` implementation.
pub fn scoped_lease_key(scope: LeaseScope, owner: &str, project: &str, name: &str) -> String {
    format!("lease/{}/{}/{}/{}", scope.as_str(), owner, project, name)
}
