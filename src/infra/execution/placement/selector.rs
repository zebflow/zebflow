//! Runner selector model used by placement policies.

use serde::{Deserialize, Serialize};

/// Declarative runner selector.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RunnerSelector {
    /// Tags that a runner must contain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
