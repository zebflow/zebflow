use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::platform::services::library::LibraryManifestSpec;

/// Canonical manifest for one embedded or distributable RWE library.
pub struct LibraryManifestContract;

impl PlatformContract for LibraryManifestContract {
    type Spec = LibraryManifestSpec;
    const KIND: ContractKind = ContractKind::LibraryManifest;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != spec.name {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match library name '{}'",
                metadata.name, spec.name
            )));
        }
        if spec.name.trim().is_empty() {
            return Err(ContractError::invalid("library name must not be empty"));
        }
        if spec.versions.is_empty() {
            return Err(ContractError::invalid(format!(
                "library '{}' must define at least one version",
                spec.name
            )));
        }
        for (key, version) in &spec.versions {
            if key.trim().is_empty() || version.entry.trim().is_empty() {
                return Err(ContractError::invalid(format!(
                    "library '{}' has an empty version key or entry",
                    spec.name
                )));
            }
            if version.source != "offline" && version.source != "online" {
                return Err(ContractError::invalid(format!(
                    "library '{}' version '{}' has unsupported source '{}'",
                    spec.name, key, version.source
                )));
            }
        }
        Ok(())
    }
}
