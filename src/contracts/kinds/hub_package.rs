use serde_json::Value;

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};

/// Canonical Hub package artifact contract.
pub struct HubPackageContract;

impl PlatformContract for HubPackageContract {
    type Spec = Value;
    const KIND: ContractKind = ContractKind::HubPackage;

    fn validate(_metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if !spec.is_object() {
            return Err(ContractError::invalid(
                "HubPackage spec must be a JSON object",
            ));
        }
        Ok(())
    }
}
