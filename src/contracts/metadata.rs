use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ContractError;

/// Common identity and release metadata for every platform contract.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractMetadata {
    /// Stable object name within the owning scope.
    pub name: String,
    /// Optional independent package or artifact release version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional content digest, normally `sha256:<hex>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Small non-secret extension values.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
}

impl ContractMetadata {
    /// Creates metadata for a named contract object.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ContractError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ContractError::invalid("metadata.name must not be empty"));
        }
        if name.len() > 256 || name.chars().any(char::is_control) {
            return Err(ContractError::invalid(
                "metadata.name must be at most 256 bytes and contain no control characters",
            ));
        }
        if let Some(version) = &self.version
            && (version.trim().is_empty() || version.chars().any(char::is_control))
        {
            return Err(ContractError::invalid(
                "metadata.version must be non-empty and contain no control characters",
            ));
        }
        if let Some(digest) = &self.digest {
            let Some(hex) = digest.strip_prefix("sha256:") else {
                return Err(ContractError::invalid(
                    "metadata.digest must use the sha256:<hex> form",
                ));
            };
            if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(ContractError::invalid(
                    "metadata.digest must contain exactly 64 hexadecimal digits",
                ));
            }
        }
        Ok(())
    }
}
