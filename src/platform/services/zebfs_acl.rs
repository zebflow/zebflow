//! Platform-owned persistence for backend-neutral ZebFS access rules.

use crate::contracts::kinds::ZebFsAclContract;
use crate::contracts::{ContractMetadata, read_optional_contract, write_contract};
use crate::zebfs::acl::{ACL_MANIFEST_PATH, ZebFsAclManifest};
use crate::zebfs::{LocalZebFs, ZebFsAccess, ZebFsAclScope, ZebFsError};

pub fn read(zebfs: &LocalZebFs) -> Result<ZebFsAclManifest, ZebFsError> {
    let path = zebfs.root().join(ACL_MANIFEST_PATH);
    match read_optional_contract::<ZebFsAclContract>(&path) {
        Ok(document) => Ok(document.map(|value| value.spec).unwrap_or_default()),
        Err(err) => match read_pre_contract_manifest(&path) {
            Some(manifest) => Ok(manifest),
            None => Err(ZebFsError::new(
                "ZEBFS_ACL_READ",
                format!("{} ({})", err, err.category()),
            )),
        },
    }
}

/// Reads an ACL written before this document carried an envelope.
///
/// The pre-contract shape is bare: `{"version":N,"rules":{…}}`. An ACL is the
/// one document here that cannot be regenerated — it records a human decision
/// — so `kinds/zebfs-acl/README.md` accepts the old shape on read and lets the
/// next change rewrite it enveloped. Refusing it instead turns every `/fs/…`
/// read into a 500 with no repair path, because `set_access` reads before it
/// writes and so cannot fix a file it will not open.
///
/// Only the bare shape is accepted, and the drifted `version` counter the
/// envelope replaced is dropped. A document carrying `apiVersion` or `kind` is
/// an envelope and is judged as one, so a future `apiVersion` still refuses.
fn read_pre_contract_manifest(path: &std::path::Path) -> Option<ZebFsAclManifest> {
    let bytes = std::fs::read(path).ok()?;
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let object = value.as_object_mut()?;
    if object.contains_key("apiVersion") || object.contains_key("kind") {
        return None;
    }
    object.remove("version");
    serde_json::from_value(value).ok()
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
    fn acl_roundtrips_and_rejects_future_documents() {
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
        std::fs::write(
            &acl_path,
            br#"{"apiVersion":"zebflow.com/v9","kind":"ZebFsAcl","metadata":{"name":"a"},"spec":{"rules":{}}}"#,
        )
        .unwrap();
        assert_eq!(
            effective_access(&zebfs, "public/file.txt")
                .unwrap_err()
                .code,
            "ZEBFS_ACL_READ"
        );
    }

    /// `kinds/zebfs-acl/README.md`: the bare pre-contract shape "is accepted on
    /// read and rewritten in the enveloped form on the next change. Nobody
    /// loses a sharing setting to a format change."
    #[test]
    fn pre_contract_acl_is_read_and_rewritten_enveloped_on_the_next_change() {
        let root = tempfile::tempdir().unwrap();
        let zebfs = LocalZebFs::new(root.path().to_path_buf());
        let acl_path = root.path().join(ACL_MANIFEST_PATH);
        std::fs::create_dir_all(acl_path.parent().unwrap()).unwrap();
        std::fs::write(
            &acl_path,
            br#"{"version":1,"rules":{"shared":{"access":"public_read","scope":"prefix","updated_at":1782287499}}}"#,
        )
        .unwrap();

        // Read accepts it, and the sharing setting survives.
        assert!(is_public_read(&zebfs, "shared/photo.jpg").unwrap());
        assert!(!is_public_read(&zebfs, "private/photo.jpg").unwrap());

        // The next change rewrites it enveloped, keeping the old rule.
        set_access(
            &zebfs,
            "also-shared",
            ZebFsAccess::PublicRead,
            ZebFsAclScope::Prefix,
        )
        .unwrap();
        let rewritten: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&acl_path).unwrap()).unwrap();
        assert_eq!(rewritten["apiVersion"], "zebflow.com/v1");
        assert_eq!(rewritten["kind"], "ZebFsAcl");
        assert!(rewritten["spec"]["rules"]["shared"].is_object());
        assert!(is_public_read(&zebfs, "shared/photo.jpg").unwrap());
        assert!(is_public_read(&zebfs, "also-shared/photo.jpg").unwrap());
    }

    /// The allowance is for the bare shape only: garbage still refuses, and so
    /// does an enveloped document with an unknown root field.
    #[test]
    fn pre_contract_allowance_does_not_accept_arbitrary_documents() {
        let root = tempfile::tempdir().unwrap();
        let zebfs = LocalZebFs::new(root.path().to_path_buf());
        let acl_path = root.path().join(ACL_MANIFEST_PATH);
        std::fs::create_dir_all(acl_path.parent().unwrap()).unwrap();

        std::fs::write(&acl_path, br#"{"rules":{"a":{"access":"nope"}}}"#).unwrap();
        assert_eq!(read(&zebfs).unwrap_err().code, "ZEBFS_ACL_READ");

        std::fs::write(&acl_path, b"not json at all").unwrap();
        assert_eq!(read(&zebfs).unwrap_err().code, "ZEBFS_ACL_READ");

        std::fs::write(
            &acl_path,
            br#"{"apiVersion":"zebflow.com/v1","kind":"ZebFsAcl","metadata":{"name":"a"},"spec":{"rules":{}},"extra":1}"#,
        )
        .unwrap();
        assert_eq!(read(&zebfs).unwrap_err().code, "ZEBFS_ACL_READ");
    }
}
