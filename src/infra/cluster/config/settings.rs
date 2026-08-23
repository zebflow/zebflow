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
    /// One-time join token supplied to a worker during bootstrap.
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
            "A controller without a join token rejects every office that tries to register, \
             so the cluster never forms."
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
    pub fn missing_required_env(&self) -> Vec<&'static str> {
        let present =
            |value: &Option<String>| value.as_deref().is_some_and(|item| !item.trim().is_empty());
        let mut missing = Vec::new();
        match self.role {
            ClusterRole::Standalone => {}
            ClusterRole::Master => {
                if !present(&self.join_token) {
                    missing.push(ENV_JOIN_TOKEN);
                }
            }
            ClusterRole::Worker => {
                if !present(&self.master_url) {
                    missing.push(ENV_MASTER_URL);
                }
                if !present(&self.join_token) {
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
    pub fn validate(&self) -> Result<(), ClusterConfigError> {
        let missing = self.missing_required_env();
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
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn office_without_cluster_configuration_refuses_and_names_every_missing_variable() {
        let settings = ClusterSettings::from_lookup(ClusterRole::Worker, "", lookup_from(&[]));
        let err = settings
            .validate()
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
                (ENV_JOIN_TOKEN, "join-token"),
            ]),
        );
        // advertise_url is not set, so the listen URL passed in stands in for it.
        assert_eq!(
            settings.advertise_url.as_deref(),
            Some("http://127.0.0.1:10610")
        );
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn blank_variables_count_as_unset_rather_than_as_values() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[
                (ENV_MASTER_URL, "   "),
                (ENV_JOIN_TOKEN, "join-token"),
                (ENV_ADVERTISE_URL, ""),
            ]),
        );
        // A blank advertise URL falls back instead of silently disabling registration.
        assert_eq!(
            settings.advertise_url.as_deref(),
            Some("http://127.0.0.1:10610")
        );
        assert_eq!(
            settings.validate().expect_err("blank master url").missing,
            vec![ENV_MASTER_URL]
        );
    }

    #[test]
    fn controller_without_a_join_token_refuses_to_start() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Master,
            "http://127.0.0.1:10610",
            lookup_from(&[]),
        );
        assert_eq!(
            settings
                .validate()
                .expect_err("controller must refuse to start")
                .missing,
            vec![ENV_JOIN_TOKEN]
        );

        let configured = ClusterSettings::from_lookup(
            ClusterRole::Master,
            "http://127.0.0.1:10610",
            lookup_from(&[(ENV_JOIN_TOKEN, "join-token")]),
        );
        assert!(configured.validate().is_ok());
    }

    #[test]
    fn node_identity_variables_are_optional() {
        let settings = ClusterSettings::from_lookup(
            ClusterRole::Worker,
            "http://127.0.0.1:10610",
            lookup_from(&[
                (ENV_MASTER_URL, "http://controller:10610"),
                (ENV_JOIN_TOKEN, "join-token"),
                (ENV_NODE_ID, "office-a"),
                (ENV_NODE_LABEL, "Office A"),
            ]),
        );
        assert_eq!(settings.node_id.as_deref(), Some("office-a"));
        assert_eq!(settings.node_label.as_deref(), Some("Office A"));
        assert!(settings.validate().is_ok());
    }
}
