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

/// Whether `raw` names git's own directory, `.git`, at any depth and in any
/// case (a case-insensitive disk reads `.GIT` as `.git`).
///
/// A repository's `.git` belongs to git: a file written there is a hook, a
/// config that names a program, or a ref. Nothing a person, a package or an
/// agent asks to write in a repository goes there.
pub fn rel_path_names_git_dir(raw: &str) -> bool {
    raw.split(['/', '\\'])
        .any(|segment| segment.trim().eq_ignore_ascii_case(".git"))
}

/// Whether `path` really lies under `root`, with every symlink followed.
///
/// [`contained_rel_path`] decides containment on the string, which a symlink
/// inside the root defeats: `root/static/x` that links to `/etc` is lexically
/// inside and really outside. A path that does not exist yet is judged by its
/// nearest existing ancestor, which is where a write would land; a dangling
/// link is refused outright, because writing through it creates its target.
pub fn real_path_within(root: &Path, path: &Path) -> bool {
    let Ok(real_root) = std::fs::canonicalize(root) else {
        return false;
    };
    let mut probe = path;
    loop {
        match std::fs::canonicalize(probe) {
            Ok(real) => return real.starts_with(&real_root),
            Err(_) if probe.symlink_metadata().is_ok() => return false,
            Err(_) => match probe.parent() {
                Some(parent) => probe = parent,
                None => return false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{contained_rel_path, real_path_within, rel_path_escapes_root, rel_path_names_git_dir};

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_the_root_is_outside() {
        let base = std::env::temp_dir().join(format!("zf-real-path-{}", std::process::id()));
        let root = base.join("root");
        let outside = base.join("outside");
        std::fs::create_dir_all(root.join("static")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret"), "x").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("static/out")).unwrap();
        std::os::unix::fs::symlink(base.join("missing"), root.join("dangling")).unwrap();
        std::fs::write(root.join("static/ok.txt"), "ok").unwrap();

        assert!(real_path_within(&root, &root.join("static/ok.txt")));
        assert!(real_path_within(&root, &root.join("static/new/file.txt")));
        assert!(!real_path_within(&root, &root.join("static/out/secret")));
        assert!(!real_path_within(&root, &root.join("static/out/new.txt")));
        assert!(!real_path_within(&root, &root.join("dangling")));
        assert!(!real_path_within(&root, &root.join("dangling/x")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_git_directory_is_named_in_any_case_at_any_depth() {
        for raw in [".git", ".git/config", "a/.git/hooks/pre-commit", ".GIT/config", "a\\.Git\\x"] {
            assert!(rel_path_names_git_dir(raw), "{raw}");
        }
        for raw in [".github/workflows/ci.yml", ".gitignore", "docs/git.md", "a.git/b"] {
            assert!(!rel_path_names_git_dir(raw), "{raw}");
        }
    }

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
