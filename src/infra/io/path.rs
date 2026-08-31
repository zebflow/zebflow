//! One spelling of "a caller-supplied relative path stays inside its root".
//!
//! Several places take a path out of a request body or a pipeline payload and
//! join it onto a root directory. Each had grown its own cleaner, and the
//! cleaners disagreed: some stripped a leading `/`, some stripped `./`, none
//! removed `..`. The guard that was supposed to catch the difference —
//! `joined.starts_with(root)` — cannot: [`std::path::Path::starts_with`]
//! compares *components* without resolving them, so `root/../../etc` still
//! begins with `root` and passes.
//!
//! So containment is decided here, on the string, before any join happens.
//! [`contained_rel_path`] returns a path built only from ordinary segments:
//! joining it onto a root cannot leave that root, whatever the caller typed.
//! Traversal is dropped rather than refused, which is the rule
//! `normalize_template_rel_path` already applied to template identities — a
//! caller who types `../../secrets/key` lands at `secrets/key` inside their own
//! root instead of reaching another tenant's disk.

use std::path::{Component, Path};

/// Reduces `raw` to a relative path that cannot escape whatever root it is
/// joined onto.
///
/// Backslashes become separators first, so a Windows-flavoured `..\..\etc` is
/// seen as traversal rather than as one long file name. Empty segments, `.`,
/// and `..` are dropped; a leading `/` or drive prefix is dropped with them.
/// The result may be empty, which means "the root itself" — callers that need
/// a file decide whether that is an error.
pub fn contained_rel_path(raw: &str) -> String {
    let unified = raw.trim().replace('\\', "/");
    Path::new(&unified)
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => {
                let text = segment.to_string_lossy();
                let text = text.trim();
                (!text.is_empty()).then(|| text.to_string())
            }
            // `.` is redundant, `..` is traversal, and a root or drive prefix
            // would make the join absolute. None of them may survive.
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether `raw` names anything other than a plain descendant of its root.
///
/// This is the same question [`contained_rel_path`] answers by rewriting, for
/// the callers that would rather refuse a hostile path than quietly relocate
/// it. Trailing or duplicated separators are not an escape; `..`, an absolute
/// path, a drive prefix, and an embedded null byte are.
pub fn rel_path_escapes_root(raw: &str) -> bool {
    let unified = raw.trim().replace('\\', "/");
    if unified.contains('\0') {
        return true;
    }
    Path::new(&unified).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{contained_rel_path, rel_path_escapes_root};

    #[test]
    fn traversal_cannot_survive_normalization() {
        for raw in [
            "../../../../tmp/evil",
            "billing/../../../../tmp/evil",
            "/etc/passwd",
            "..\\..\\tmp\\evil",
            "./././../secret",
        ] {
            let cleaned = contained_rel_path(raw);
            assert!(
                !cleaned.contains(".."),
                "'{raw}' normalized to '{cleaned}', which still traverses"
            );
            assert!(
                !cleaned.starts_with('/'),
                "'{raw}' normalized to '{cleaned}', which is absolute"
            );
        }
    }

    #[test]
    fn ordinary_paths_are_left_alone() {
        assert_eq!(
            contained_rel_path("pipelines/api/foo.zf.json"),
            "pipelines/api/foo.zf.json"
        );
        assert_eq!(
            contained_rel_path("./pipelines/foo.zf.json"),
            "pipelines/foo.zf.json"
        );
        assert_eq!(
            contained_rel_path("/pipelines//foo.zf.json"),
            "pipelines/foo.zf.json"
        );
        assert_eq!(contained_rel_path("  spaced/name.txt  "), "spaced/name.txt");
        assert_eq!(contained_rel_path(""), "");
        assert_eq!(contained_rel_path("../.."), "");
    }

    #[test]
    fn a_joined_normalized_path_stays_under_its_root() {
        let root = std::path::Path::new("/srv/zebflow/users/alice/shop/repo");
        let joined = root.join(contained_rel_path(
            "../../../bob/private/repo/src/steal.zf.json",
        ));
        assert!(joined.starts_with(root));
        assert!(!joined.to_string_lossy().contains(".."));
    }

    #[test]
    fn escape_detection_matches_normalization() {
        assert!(rel_path_escapes_root("../up"));
        assert!(rel_path_escapes_root("/absolute"));
        assert!(rel_path_escapes_root("a/../../b"));
        assert!(rel_path_escapes_root("a\0b"));
        assert!(!rel_path_escapes_root("a/b/c.txt"));
        assert!(!rel_path_escapes_root("./a/b"));
        assert!(!rel_path_escapes_root("a//b/"));
    }
}
