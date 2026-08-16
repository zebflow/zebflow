use serde::{Deserialize, Serialize};

/// The only platform contract API version accepted by this release.
pub const CONTRACT_API_VERSION: &str = "zebflow.com/v1";

/// Closed list of authoritative Zebflow contract kinds across all domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractKind {
    ProjectConfiguration,
    ProjectManifest,
    Pipeline,
    DependencyLock,
    NodeDefinition,
    NodeBundle,
    FileRef,
    ZebFsAcl,
    DatabaseSchema,
    ProjectBundle,
    RuntimeBundle,
    HubPackage,
    LibraryManifest,
    MapPublishManifest,
    InvocationRecord,
}

impl ContractKind {
    /// Canonical serialized kind name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProjectConfiguration => "ProjectConfiguration",
            Self::ProjectManifest => "ProjectManifest",
            Self::Pipeline => "Pipeline",
            Self::DependencyLock => "DependencyLock",
            Self::NodeDefinition => "NodeDefinition",
            Self::NodeBundle => "NodeBundle",
            Self::FileRef => "FileRef",
            Self::ZebFsAcl => "ZebFsAcl",
            Self::DatabaseSchema => "DatabaseSchema",
            Self::ProjectBundle => "ProjectBundle",
            Self::RuntimeBundle => "RuntimeBundle",
            Self::HubPackage => "HubPackage",
            Self::LibraryManifest => "LibraryManifest",
            Self::MapPublishManifest => "MapPublishManifest",
            Self::InvocationRecord => "InvocationRecord",
        }
    }

    /// Parses a canonical kind name. Aliases are intentionally unsupported.
    pub fn parse(value: &str) -> Option<Self> {
        ALL_CONTRACT_DESCRIPTORS
            .iter()
            .find(|descriptor| descriptor.kind.as_str() == value)
            .map(|descriptor| descriptor.kind)
    }
}

impl std::fmt::Display for ContractKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Registry information used by documentation and architecture tests.
#[derive(Debug, Clone, Copy)]
pub struct ContractDescriptor {
    pub kind: ContractKind,
    pub owner: &'static str,
    pub boundary: &'static str,
    pub representation: ContractRepresentation,
}

/// How a registered contract is represented at its boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractRepresentation {
    /// Canonical `apiVersion/kind/metadata/spec` document.
    Envelope,
    /// Small discriminated value carried inside node JSON.
    InlinePayload,
    /// Typed row stored under a database schema migration contract.
    DatabaseRecord,
    /// Name reserved for a future format; no reader may accept it yet.
    Reserved,
}

const ALL_CONTRACT_DESCRIPTORS: &[ContractDescriptor] = &[
    envelope(ContractKind::ProjectConfiguration, "platform", "persisted"),
    reserved(ContractKind::ProjectManifest, "platform", "persisted"),
    envelope(ContractKind::Pipeline, "pipeline", "persisted"),
    envelope(ContractKind::DependencyLock, "platform", "persisted"),
    envelope(ContractKind::NodeDefinition, "pipeline", "normalized"),
    envelope(ContractKind::NodeBundle, "platform", "transferred"),
    inline(ContractKind::FileRef, "pipeline", "payload"),
    envelope(ContractKind::ZebFsAcl, "zebfs", "persisted"),
    envelope(ContractKind::DatabaseSchema, "platform", "persisted"),
    envelope(ContractKind::ProjectBundle, "platform", "transferred"),
    envelope(ContractKind::RuntimeBundle, "execution", "transferred"),
    envelope(ContractKind::HubPackage, "platform", "transferred"),
    envelope(ContractKind::LibraryManifest, "rwe", "persisted"),
    envelope(ContractKind::MapPublishManifest, "mapserver", "persisted"),
    database_record(ContractKind::InvocationRecord, "pipeline", "persisted"),
];

const fn descriptor(
    kind: ContractKind,
    owner: &'static str,
    boundary: &'static str,
    representation: ContractRepresentation,
) -> ContractDescriptor {
    ContractDescriptor {
        kind,
        owner,
        boundary,
        representation,
    }
}

const fn envelope(
    kind: ContractKind,
    owner: &'static str,
    boundary: &'static str,
) -> ContractDescriptor {
    descriptor(kind, owner, boundary, ContractRepresentation::Envelope)
}

const fn inline(
    kind: ContractKind,
    owner: &'static str,
    boundary: &'static str,
) -> ContractDescriptor {
    descriptor(kind, owner, boundary, ContractRepresentation::InlinePayload)
}

const fn database_record(
    kind: ContractKind,
    owner: &'static str,
    boundary: &'static str,
) -> ContractDescriptor {
    descriptor(
        kind,
        owner,
        boundary,
        ContractRepresentation::DatabaseRecord,
    )
}

const fn reserved(
    kind: ContractKind,
    owner: &'static str,
    boundary: &'static str,
) -> ContractDescriptor {
    descriptor(kind, owner, boundary, ContractRepresentation::Reserved)
}

/// Returns the one registered descriptor for a kind.
pub fn contract_descriptor(kind: ContractKind) -> &'static ContractDescriptor {
    ALL_CONTRACT_DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.kind == kind)
        .expect("every ContractKind must be registered")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_registry_entry_and_roundtrips() {
        let mut kinds = std::collections::HashSet::new();
        for descriptor in ALL_CONTRACT_DESCRIPTORS {
            assert!(kinds.insert(descriptor.kind));
            assert_eq!(
                ContractKind::parse(descriptor.kind.as_str()),
                Some(descriptor.kind)
            );
            assert!(!descriptor.owner.is_empty());
            assert!(!descriptor.boundary.is_empty());
        }
        assert_eq!(kinds.len(), 15);
        assert_eq!(
            ALL_CONTRACT_DESCRIPTORS
                .iter()
                .filter(|item| item.representation == ContractRepresentation::Reserved)
                .count(),
            1
        );
    }
}
