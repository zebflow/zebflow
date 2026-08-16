use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{ContractError, ContractKind, ContractMetadata};

/// Typed result of parsing one canonical platform contract document.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDocument<T> {
    pub api_version: &'static str,
    pub kind: &'static str,
    pub metadata: ContractMetadata,
    pub spec: T,
}

/// Marker implemented by each registered platform contract adapter.
pub trait PlatformContract {
    type Spec: Serialize + DeserializeOwned;

    const KIND: ContractKind;

    /// Domain semantic validation after strict envelope and typed decoding.
    fn validate(_metadata: &ContractMetadata, _spec: &Self::Spec) -> Result<(), ContractError> {
        Ok(())
    }
}
