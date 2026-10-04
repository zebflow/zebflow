//! Every platform template must parse, and must declare what it uses.
//!
//! The server compiles all of them at startup and panics on the first one that
//! does not, which costs a full rebuild to discover and names only the page,
//! not the line. This test reads the same files straight from the source tree
//! and reports every broken one at once, in about a second.

use std::path::{Path, PathBuf};

fn template_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "node_modules") {
                continue;
            }
            template_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e == "tsx" || e == "ts")
        {
            out.push(path);
        }
    }
}

/// Every platform template's problems, as `file:line:column: message`, from
/// the same check `file_write` runs on a project file
/// (`zebflow::rwe::core::source_check`); `keep` picks the kind of problem.
fn template_problems(keep: impl Fn(&str) -> bool) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    template_files(&root, &mut files);
    assert!(!files.is_empty(), "no templates found under {}", root.display());
    files.sort();
    let mut out = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("read template");
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        let tsx = path.extension().is_some_and(|e| e == "tsx");
        for problem in zebflow::rwe::core::source_check::check_template_source(&source, tsx) {
            if keep(&problem.message) {
                out.push(format!("{rel}:{problem}"));
            }
        }
    }
    out
}

#[test]
fn every_platform_template_parses() {
    let broken = template_problems(|message| message.starts_with("does not parse"));
    assert!(
        broken.is_empty(),
        "{} template problem(s) — files that do not parse:\n{}",
        broken.len(),
        broken.join("\n")
    );
}

/// Every name a template file uses must be declared in it or imported into it.
///
/// The RWE compiler inlines component files into one flat bundle, so a
/// component that uses a binding it never imported still runs — it borrows the
/// entry page's. That works until the page stops holding that binding, at
/// which point the component throws `X is not defined` in the browser, with
/// nothing failing at build time. This is the check that turns that into a
/// test failure.
#[test]
fn no_template_borrows_a_binding_it_never_declared() {
    let borrowed = template_problems(|message| message.contains("is used but never declared"));
    assert!(
        borrowed.is_empty(),
        "{} binding(s) used but never declared:\n{}",
        borrowed.len(),
        borrowed.join("\n")
    );
}

/// No template may declare a binding named `h` where JSX appears.
///
/// The compiler lowers every JSX element to a call of `h(...)`. A local named
/// `h` — `hosts.map((h) => <option>{h}</option>)` — shadows the factory inside
/// its own scope, the server markup shows `<!-- RWE component error: TypeError:
/// h is not a function -->`, and the response is still 200. The Addressing
/// tab shipped exactly that; this refuses it at test time.
#[test]
fn no_template_declares_a_binding_named_h() {
    let offenders = template_problems(|message| message.contains("binding named `h`"));
    assert!(
        offenders.is_empty(),
        "{} template(s) declare a binding named `h`, the JSX factory:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
