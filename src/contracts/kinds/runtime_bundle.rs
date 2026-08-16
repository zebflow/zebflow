use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::platform::model::ProjectRuntimeMaterializationRequest;

/// Canonical office-to-office project materialization request.
pub struct RuntimeBundleContract;

impl PlatformContract for RuntimeBundleContract {
    type Spec = ProjectRuntimeMaterializationRequest;
    const KIND: ContractKind = ContractKind::RuntimeBundle;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        let identity = &spec.bundle.identity;
        if identity.owner.trim().is_empty() || identity.project.trim().is_empty() {
            return Err(ContractError::invalid(
                "RuntimeBundle identity owner and project must not be empty",
            ));
        }
        if metadata.name != identity.project {
            return Err(ContractError::invalid(format!(
                "metadata.name '{}' must match project '{}'",
                metadata.name, identity.project
            )));
        }
        Ok(())
    }
}
