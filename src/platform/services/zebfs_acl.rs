//! Platform-owned persistence for backend-neutral ZebFS access rules.

use crate::contracts::kinds::ZebFsAclContract;
use crate::contracts::{ContractMetadata, read_optional_contract, write_contract};
use crate::zebfs::acl::{ACL_MANIFEST_PATH, ZebFsAclManifest};
use crate::zebfs::{LocalZebFs, ZebFsAccess, ZebFsAclScope, ZebFsError};

pub fn read(zebfs: &LocalZebFs) -> Result<ZebFsAclManifest, ZebFsError> {
    let path = zebfs.root().join(ACL_MANIFEST_PATH);
    read_optional_contract::<ZebFsAclContract>(&path)
        .map(|document| document.map(|value| value.spec).unwrap_or_default())
        .map_err(|err| ZebFsError::new("ZEBFS_ACL_READ", format!("{} ({})", err, err.category())))
}

pub fn write(zebfs: &LocalZebFs, manifest: &ZebFsAclManifest) -> Result<(), ZebFsError> {
    let path = zebfs.root().join(ACL_MANIFEST_PATH);
    write_contract::<ZebFsAclContract>(
        &path,
        ContractMetadata::named("access-control"),
        manifest.clone(),
    )
    .map_err(|err| ZebFsError::new("ZEBFS_ACL_WRITE", format!("{} ({})", err, err.category())))
}

pub fn effective_access(zebfs: &LocalZebFs, path: &str) -> Result<ZebFsAccess, ZebFsError> {
    read(zebfs)?.effective_access(path)
}

pub fn is_public_read(zebfs: &LocalZebFs, path: &str) -> Result<bool, ZebFsError> {
    Ok(effective_access(zebfs, path)? == ZebFsAccess::PublicRead)
}

pub fn set_access(
    zebfs: &LocalZebFs,
    path: &str,
    access: ZebFsAccess,
    scope: ZebFsAclScope,
) -> Result<String, ZebFsError> {
    let mut manifest = read(zebfs)?;
    let normalized = manifest.set_rule(path, access, scope)?;
    write(zebfs, &manifest)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_roundtrips_and_rejects_old_or_future_documents() {
        let root = tempfile::tempdir().unwrap();
        let zebfs = LocalZebFs::new(root.path().to_path_buf());
        set_access(
            &zebfs,
            "public",
            ZebFsAccess::PublicRead,
            ZebFsAclScope::Prefix,
        )
        .unwrap();
        assert!(is_public_read(&zebfs, "public/file.txt").unwrap());

        let acl_path = root.path().join(ACL_MANIFEST_PATH);
        std::fs::write(&acl_path, br#"{"version":2,"rules":{}}"#).unwrap();
        assert_eq!(
            effective_access(&zebfs, "public/file.txt")
                .unwrap_err()
                .code,
            "ZEBFS_ACL_READ"
        );
    }
}
