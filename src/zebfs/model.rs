use std::time::SystemTime;

/// One object read from ZebFS.
#[derive(Debug, Clone)]
pub struct ZebFsObject {
    pub path: String,
    pub bytes: Vec<u8>,
    pub stat: ZebFsStat,
}

/// Metadata for one ZebFS path.
#[derive(Debug, Clone)]
pub struct ZebFsStat {
    pub path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub kind: ZebFsEntryKind,
}

/// One entry returned by list-prefix.
#[derive(Debug, Clone)]
pub struct ZebFsEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub kind: ZebFsEntryKind,
}

/// ZebFS entry kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZebFsEntryKind {
    Object,
    Prefix,
}
