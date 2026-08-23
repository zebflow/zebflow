//! Cluster role definitions.
//!
//! `Master` and `Worker` are the internal enum names used by the first clustered
//! implementation. The product vocabulary is `controller` and `office`, and those
//! are the words the command line, the help text, and `docs/contracts/interface.md`
//! use.
//!
//! The binary also still answers to `master` and `worker`. They are deprecated
//! spellings, not a second vocabulary: each names exactly one canonical word,
//! announces itself when used, and is documented rather than secret.

use serde::{Deserialize, Serialize};

/// Runtime role of the current process.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClusterRole {
    /// One all-in-one process: current Zebflow behavior.
    #[default]
    Standalone,
    /// Control-plane role, surfaced as `controller`.
    Master,
    /// Execution-plane role, surfaced as `office`.
    Worker,
}

/// A server-mode word resolved to the role it selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerMode {
    /// Role the process will take.
    pub role: ClusterRole,
    /// Canonical word to use instead, set only when a deprecated spelling was typed.
    pub deprecated_alias_of: Option<&'static str>,
}

impl ClusterRole {
    /// Every role, in the order the help text lists them.
    pub const ALL: [ClusterRole; 3] = [Self::Standalone, Self::Master, Self::Worker];

    /// Canonical command-line word for this role.
    pub fn mode_word(self) -> &'static str {
        match self {
            ClusterRole::Standalone => "standalone",
            ClusterRole::Master => "controller",
            ClusterRole::Worker => "office",
        }
    }

    /// Deprecated command-line spelling still accepted for this role.
    ///
    /// These predate the product vocabulary and were never documented, so no one
    /// could have learned them except by reading the source. They stay accepted
    /// because a word someone has already typed into a script outlives the code
    /// that chose it, and they warn so the pair does not become permanent.
    pub fn deprecated_mode_word(self) -> Option<&'static str> {
        match self {
            ClusterRole::Standalone => None,
            ClusterRole::Master => Some("master"),
            ClusterRole::Worker => Some("worker"),
        }
    }

    /// Resolve a server-mode argument to the role it selects.
    pub fn from_mode_arg(arg: &str) -> Option<ServerMode> {
        for role in Self::ALL {
            if arg == role.mode_word() {
                return Some(ServerMode {
                    role,
                    deprecated_alias_of: None,
                });
            }
            if role.deprecated_mode_word() == Some(arg) {
                return Some(ServerMode {
                    role,
                    deprecated_alias_of: Some(role.mode_word()),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_mode_words_select_their_role_without_deprecation() {
        for (arg, role) in [
            ("standalone", ClusterRole::Standalone),
            ("controller", ClusterRole::Master),
            ("office", ClusterRole::Worker),
        ] {
            let mode = ClusterRole::from_mode_arg(arg).expect("canonical mode word");
            assert_eq!(mode.role, role);
            assert_eq!(mode.deprecated_alias_of, None);
        }
    }

    #[test]
    fn legacy_mode_words_still_select_their_role_and_name_the_canonical_word() {
        let controller = ClusterRole::from_mode_arg("master").expect("legacy mode word");
        assert_eq!(controller.role, ClusterRole::Master);
        assert_eq!(controller.deprecated_alias_of, Some("controller"));

        let office = ClusterRole::from_mode_arg("worker").expect("legacy mode word");
        assert_eq!(office.role, ClusterRole::Worker);
        assert_eq!(office.deprecated_alias_of, Some("office"));
    }

    #[test]
    fn unknown_mode_words_resolve_to_nothing() {
        assert_eq!(ClusterRole::from_mode_arg("Controller"), None);
        assert_eq!(ClusterRole::from_mode_arg("leader"), None);
        assert_eq!(ClusterRole::from_mode_arg(""), None);
    }
}
