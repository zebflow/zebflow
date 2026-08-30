//! Cluster settings used by the unified `zebflow` binary.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::role::ClusterRole;

/// Environment variable each cluster setting is read from.
///
/// The names live beside the fields because a startup refusal has to name the
/// variable an operator can set, not the struct field they cannot see.
pub const ENV_NODE_ID: &str = "ZEBFLOW_CLUSTER_NODE_ID";
/// Environment variable for [`ClusterSettings::node_label`].
pub const ENV_NODE_LABEL: &str = "ZEBFLOW_CLUSTER_NODE_LABEL";
/// Environment variable for [`ClusterSettings::master_url`].
pub const ENV_MASTER_URL: &str = "ZEBFLOW_CLUSTER_MASTER_URL";
/// Environment variable for [`ClusterSettings::advertise_url`].
pub const ENV_ADVERTISE_URL: &str = "ZEBFLOW_CLUSTER_ADVERTISE_URL";
/// Environment variable for [`ClusterSettings::join_token`].
///
/// An office only. The controller issues tokens and verifies them against its
/// own records, so it holds no token of its own (`offices.md` §8: a shared
/// environment secret is not a token).
pub const ENV_JOIN_TOKEN: &str = "ZEBFLOW_CLUSTER_JOIN_TOKEN";

/// Cluster bootstrap and runtime settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterSettings {
    /// Runtime role for this process.
    pub role: ClusterRole,
    /// Stable node id when running as master or worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Human-readable node label for inventory and operator UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_label: Option<String>,
    /// Master URL used by workers to join and connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_url: Option<String>,
    /// Public/internal base URL this node advertises to the control plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advertise_url: Option<String>,
    /// Per-office join token supplied to an office for its first join.
    ///
    /// Shaped `zfjoin2:<office_id>:<controller_verify_key>:<secret>`, minted by
    /// the controller. After the first join the office reads what it stored in
    /// its own data root and this variable may be unset; supplying one that
    /// disagrees with the stored token is a refusal, never a silent overwrite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_token: Option<String>,
}

impl Default for ClusterSettings {
    fn default() -> Self {
        Self {
            role: ClusterRole::Standalone,
            node_id: None,
            node_label: None,
            master_url: None,
            advertise_url: None,
            join_token: None,
        }
    }
}

/// A role the process was asked to take and is not configured to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterConfigError {
    /// Role that was requested.
    pub role: ClusterRole,
    /// Every environment variable the role needs and does not have.
    pub missing: Vec<&'static str>,
}

impl fmt::Display for ClusterConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`zebflow {}` is missing required cluster configuration: {}. {}",
            self.role.mode_word(),
            self.missing.join(", "),
            incomplete_consequence(self.role),
        )
    }
}

impl std::error::Error for ClusterConfigError {}

/// What the process would do if it started in this incomplete state.
///
/// Stated in the error because "set this variable" answers *how* and the refusal
/// still has to answer *why a warning was not enough*.
fn incomplete_consequence(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Standalone => "A standalone process needs no cluster configuration.",
        ClusterRole::Master => {
            "A controller mints and verifies per-office tokens from its own catalog, so it \
             needs no cluster variables of its own."
        }
        ClusterRole::Worker => {
            "An office that cannot reach a controller would serve traffic while belonging to \
             no cluster, which looks healthy and is not."
        }
    }
}

impl ClusterSettings {
    /// Read cluster settings for `role` from a variable lookup.
    ///
    /// The lookup is a parameter so this is testable without mutating the process
    /// environment, which no two tests can do at the same time.
    ///
    /// An empty or whitespace-only value counts as unset. An exported but blank
    /// variable is a misconfiguration, and reading it as a value is how a blank
    /// advertise URL used to skip the fallback below and disable registration.
    pub fn from_lookup(
        role: ClusterRole,
        default_advertise_url: &str,
        lookup: impl Fn(&str) -> Option<String>,
    ) -> Self {
        let value = |name: &str| lookup(name).filter(|item| !item.trim().is_empty());
        Self {
            role,
            node_id: value(ENV_NODE_ID),
            node_label: value(ENV_NODE_LABEL),
            master_url: value(ENV_MASTER_URL),
            advertise_url: value(ENV_ADVERTISE_URL)
                .or_else(|| Some(default_advertise_url.to_string())),
            join_token: value(ENV_JOIN_TOKEN),
        }
    }

    /// Read cluster settings for `role` from the process environment.
    pub fn from_env(role: ClusterRole, default_advertise_url: &str) -> Self {
        Self::from_lookup(role, default_advertise_url, |name| std::env::var(name).ok())
    }

    /// Environment variables this role needs and does not have.
    ///
    /// Returned all at once, in the order an operator would fill them in, because
    /// learning one missing name per restart turns a single edit into three deploys.
    ///
    /// `stored_join_token` is whether this data root already holds
    /// `platform/office-join-token`. It is a parameter and not a field because
    /// it is a fact about the disk rather than about the configuration, and it
    /// is consulted because both `zeb help` and `interface.md` §5 promise
    /// `ZEBFLOW_CLUSTER_JOIN_TOKEN` is needed "for its **first** join only". A
    /// joined office that could not restart after a reboot without an operator
    /// re-supplying the variable would make that sentence false and would make
    /// every unattended restart a manual one. Disagreement between the variable
    /// and the stored token is still a refusal, naming both — that check lives
    /// where the two values are, in `resolve_office_identity`.
    pub fn missing_required_env(&self, stored_join_token: bool) -> Vec<&'static str> {
        let present =
            |value: &Option<String>| value.as_deref().is_some_and(|item| !item.trim().is_empty());
        let mut missing = Vec::new();
        match self.role {
            ClusterRole::Standalone => {}
            // A controller needs nothing. It mints a token per office and
            // verifies each against the record it created, which is what makes
            // one office revocable on its own (`offices.md` §8).
            ClusterRole::Master => {}
            ClusterRole::Worker => {
                if !present(&self.master_url) {
                    missing.push(ENV_MASTER_URL);
                }
                if !present(&self.join_token) && !stored_join_token {
                    missing.push(ENV_JOIN_TOKEN);
                }
                if !present(&self.advertise_url) {
                    missing.push(ENV_ADVERTISE_URL);
                }
            }
        }
        missing
    }

    /// Refuse a role this process is not configured to perform.
    pub fn validate(&self, stored_join_token: bool) -> Result<(), ClusterConfigError> {
        let missing = self.missing_required_env(stored_join_token);
        if missing.is_empty() {
            return Ok(());
        }
        Err(ClusterConfigError {
            role: self.role,
            missing,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup_from(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        move |name: &str| {
            owned
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    #[test]
    fn standalone_needs_no_cluster_configuration() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Standalone,
            "http://127.0.0.1:10610",
            lookup_from(&[]),
        );
        assert!(settings.validate(false).is_ok());
    }

    #[test]
    fn office_without_cluster_configuration_refuses_and_names_every_missing_variable() {
        let settings = ClusterSettings::from_lookup(ClusterRole::Worker, "", lookup_from(&[]));
        let err = settings
            .validate(false)
            .expect_err("office must refuse to start");
        assert_eq!(
            err.missing,
            vec![ENV_MASTER_URL, ENV_JOIN_TOKEN, ENV_ADVERTISE_URL]
        );
        let message = err.to_string();
        for name in [ENV_MASTER_URL, ENV_JOIN_TOKEN, ENV_ADVERTISE_URL] {
            assert!(
                message.contains(name),
                "message must name {name}: {message}"
            );
        }
    }

    #[test]
    fn office_with_complete_configuration_starts() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[
                (ENV_MASTER_URL, "http://controller:10610"),
                (ENV_JOIN_TOKEN, "zfjoin2:office-a:key:secret"),
            ]),
        );
        // advertise_url is not set, so the listen URL passed in stands in for it.
        assert_eq!(
            settings.advertise_url.as_deref(),
            Some("http://127.0.0.1:10610")
        );
        assert!(settings.validate(false).is_ok());
    }

    /// `interface.md` §5 and `zeb help` both say the variable is for a first
    /// join only. Without this the promise was false: a joined office that
    /// rebooted refused to start until somebody re-supplied a credential it
    /// already held on disk, so no office could restart unattended.
    #[test]
    fn a_stored_token_satisfies_startup_with_the_variable_gone() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[(ENV_MASTER_URL, "http://controller:10610")]),
        );
        assert_eq!(
            settings.missing_required_env(false),
            vec![ENV_JOIN_TOKEN],
            "a first join still needs the variable"
        );
        assert!(
            settings.missing_required_env(true).is_empty(),
            "a joined office restarts on what it stored"
        );
        assert!(settings.validate(true).is_ok());

        // The stored token answers only for the token. Everything else the
        // role needs is still named.
        let bare = ClusterSettings::from_lookup(ClusterRole::Worker, "", lookup_from(&[]));
        assert_eq!(
            bare.missing_required_env(true),
            vec![ENV_MASTER_URL, ENV_ADVERTISE_URL]
        );
    }

    #[test]
    fn blank_variables_count_as_unset_rather_than_as_values() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[
                (ENV_MASTER_URL, "   "),
                (ENV_JOIN_TOKEN, "zfjoin2:office-a:key:secret"),
                (ENV_ADVERTISE_URL, ""),
            ]),
        );
        // A blank advertise URL falls back instead of silently disabling registration.
        assert_eq!(
            settings.advertise_url.as_deref(),
            Some("http://127.0.0.1:10610")
        );
        assert_eq!(
            settings
                .validate(false)
                .expect_err("blank master url")
                .missing,
            vec![ENV_MASTER_URL]
        );
    }

    #[test]
    fn a_controller_holds_no_token_of_its_own() {
        // The shared environment secret was the defect `offices.md` §8 names:
        // possession was membership, with no way to revoke one office. A
        // controller now issues per-office tokens and verifies them against
        // its own records, so it requires no cluster variable at all.
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Master,
            "http://127.0.0.1:10610",
            lookup_from(&[]),
        );
        assert!(settings.missing_required_env(false).is_empty());
        assert!(settings.validate(false).is_ok());
    }

    #[test]
    fn node_identity_variables_are_optional() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[
                (ENV_MASTER_URL, "http://controller:10610"),
                (ENV_JOIN_TOKEN, "zfjoin2:office-a:key:secret"),
                (ENV_NODE_ID, "office-a"),
                (ENV_NODE_LABEL, "Office A"),
            ]),
        );
        assert_eq!(settings.node_id.as_deref(), Some("office-a"));
        assert_eq!(settings.node_label.as_deref(), Some("Office A"));
        assert!(settings.validate(false).is_ok());
    }
}
