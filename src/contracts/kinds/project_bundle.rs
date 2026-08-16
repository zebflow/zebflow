use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::platform::model::ProjectTransferManifest;

/// Canonical manifest embedded in project transfer archives.
pub struct ProjectBundleContract;

impl PlatformContract for ProjectBundleContract {
    type Spec = ProjectTransferManifest;
    const KIND: ContractKind = ContractKind::ProjectBundle;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if spec.owner.trim().is_empty() || spec.project.trim().is_empty() {
            return Err(ContractError::invalid(
                "ProjectBundle owner and project must not be empty",
            ));
        }
        if metadata.name != spec.project {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match project '{}'",
                metadata.name, spec.project
            )));
        }
        Ok(())
    }
}
