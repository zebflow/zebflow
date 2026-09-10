//! Help describes the implementation. It does not define it.
//!
//! `pipeline/nodes` is generated from `builtin_node_definitions()` and cannot
//! lie. The 55 hand-written files can say anything, and nothing checked them —
//! so `__status`, `__redirect`, `__body` and `__notfound` were taught as
//! "special script output keys" across ten files, with zero lines of
//! implementation anywhere.
//!
//! That is not a stale document. It was never connected to anything. And the
//! help is compiled into the binary and fed to the assistant *in full* through
//! `format_for_system_prompt()`, so the agent that writes pipelines for users
//! was instructed to emit guards that silently do nothing.
//!
//! This test is the connection: a marker the help teaches must exist in the
//! code, or the build fails naming both.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_tree(root: &Path, ext: &[&str], skip: &[&str]) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let display = path.to_string_lossy().to_string();
            if skip.iter().any(|s| display.contains(s)) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| ext.contains(&e))
            {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push((path, text));
                }
            }
        }
    }
    out
}

/// Every `__marker` the help teaches must appear somewhere in the code.
///
/// A marker is a promise to the reader — and to the assistant, which receives
/// every one of these files as instructions — that returning that key from a
/// script does something. If no code mentions it, the promise is false.
#[test]
fn every_marker_the_help_teaches_exists_in_the_code() {
    let root = repo_root();
    let help = read_tree(&root.join("src/platform/help"), &["md"], &[]);
    assert!(!help.is_empty(), "no help files found");

    // Everything the code knows about, from Rust, JS and TSX alike.
    let code: String = read_tree(
        &root.join("src"),
        &["rs", "js", "tsx", "ts"],
        &["src/platform/help"],
    )
    .into_iter()
    .map(|(_, text)| text)
    .collect::<Vec<_>>()
    .join("\n");

    let mut phantoms: BTreeSet<String> = BTreeSet::new();
    for (path, text) in &help {
        let bytes = text.as_bytes();
        let mut i = 0;
        while let Some(pos) = text[i..].find("__") {
            let start = i + pos;
            let mut end = start + 2;
            while end < bytes.len()
                && (bytes[end].is_ascii_lowercase() || bytes[end] == b'_' || bytes[end].is_ascii_digit())
            {
                end += 1;
            }
            let marker = &text[start..end];
            // Two underscores and at least three more characters: a marker, not
            // markdown emphasis or a rule of dashes.
            if marker.len() > 4 && !code.contains(marker) {
                phantoms.insert(format!(
                    "{marker}  ({})",
                    path.strip_prefix(&root).unwrap_or(path).display()
                ));
            }
            i = end.max(start + 2);
        }
    }

    assert!(
        phantoms.is_empty(),
        "help teaches markers that no code implements — either build them or \
         stop teaching them:\n  {}",
        phantoms.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}
