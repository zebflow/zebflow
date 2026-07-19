//! Shared policy report primitives.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRiskLevel {
    Low,
    Medium,
    High,
}

impl PolicyRiskLevel {
    pub fn from_score(score: u32) -> Self {
        if score >= 4 {
            Self::High
        } else if score >= 2 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}
