use crate::contracts::{ContractKind, PlatformContract};
use crate::zebfs::acl::ZebFsAclManifest;

/// Canonical persisted ZebFS access-control manifest.
pub struct ZebFsAclContract;

impl PlatformContract for ZebFsAclContract {
    type Spec = ZebFsAclManifest;
    const KIND: ContractKind = ContractKind::ZebFsAcl;
}
