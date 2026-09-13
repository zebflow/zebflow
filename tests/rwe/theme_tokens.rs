//! A template names a colour role, never a colour.
//!
//! The roles are shadcn/ui's token names, verbatim, defined once for light and
//! once for dark in `styles/main.css` — see `docs/contracts/kinds/ui-theme`.
//! Two things break that and neither shows up at build time:
//!
//! - a template still speaking the retired vocabulary (`text-body`,
//!   `bg-surface-2`, `var(--color-ui-border)`) compiles to no rule at all and
//!   renders unstyled;
//! - a primitive in `components/ui/` reaching for `bg-red-500` looks right in
//!   one theme and does not move with the other.
//!
//! This test reads the template tree and names every offending line.

use std::path::{Path, PathBuf};

use regex::Regex;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e == "tsx" || e == "ts" || e == "css")
        {
            out.push(path);
        }
    }
}

const RETIRED: &[&str] = &[
    "bg", "surface", "surface-2", "surface-3", "body", "body-soft", "body-muted",
    "accent-strong", "accent-alt", "accent-alt-strong", "border-soft",
    "brand-blue", "brand-blue-ink", "brand-orange", "brand-orange-ink",
    "ui-bg", "ui-bg-muted", "ui-bg-subtle", "ui-border", "ui-border-subtle",
    "ui-text", "ui-text-muted", "ui-text-soft",
    "dark-background", "dark-border", "dark-menus", "dark-text1",
    "dark-accent1", "dark-accent2", "dark-accent3", "dark-accent4", "dark-accent5",
    "zeb-ink", "zeb-ink-soft", "zeb-ink-muted", "zeb-bg", "zeb-bg-soft", "zeb-border",
    "zeb-border-strong", "zeb-dark", "zeb-dark-2", "zeb-dark-3", "zeb-dark-border", "zeb-dark-border-2",
];

const PREFIX: &str = r"(?:border-[trblxy]|ring-offset|bg|text|border|ring|fill|stroke|from|to|via|outline|divide|placeholder|caret|decoration|shadow)";

fn alternation(items: &[&str]) -> String {
    let mut sorted: Vec<&str> = items.to_vec();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.len()));
    sorted.iter().map(|s| regex::escape(s)).collect::<Vec<_>>().join("|")
}

fn offences(files: &[PathBuf], root: &Path, pattern: &Regex) -> Vec<String> {
    let mut out = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            for m in pattern.find_iter(line) {
                out.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(root).unwrap_or(path).display(),
                    i + 1,
                    m.as_str()
                ));
            }
        }
    }
    out
}

/// The retired vocabulary compiles to nothing. Nothing may still speak it.
#[test]
fn no_template_names_a_retired_token() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    walk(&root, &mut files);
    // main.css is where the palette primitives (`--color-zeb-*`, `--color-brand-*`)
    // are defined for the two theme blocks to point at. It is the one file
    // allowed to say their names.
    files.retain(|p| p.file_name().is_some_and(|n| n != "main.css"));
    let alt = alternation(RETIRED);
    let class = Regex::new(&format!(
        r"(?:^|[^\w-])(?:[\w\[\]&>:.-]+:)*{PREFIX}-({alt})(?:/\d+)?(?:$|[^\w-])"
    ))
    .unwrap();
    let var = Regex::new(&format!(r"--color-({alt})(?:$|[^\w-])")).unwrap();

    let mut found = offences(&files, &root, &class);
    found.extend(offences(&files, &root, &var));
    assert!(
        found.is_empty(),
        "templates name retired theme tokens — use the roles in docs/contracts/kinds/ui-theme:\n  {}",
        found.join("\n  ")
    );
}

/// A primitive is themed by role. `bg-red-500` is a colour, and it stays red
/// when the theme goes dark. Pages may still carry raw palette classes for
/// now; the primitives everyone builds on may not.
#[test]
fn ui_primitives_name_no_raw_palette_colour() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    walk(&root.join("components/ui"), &mut files);
    assert!(!files.is_empty(), "no ui primitives found");
    let hues = "red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|slate|gray|zinc|neutral|stone|white|black";
    let raw = Regex::new(&format!(
        r"(?:^|[^\w-])(?:[\w\[\]&>:.-]+:)*{PREFIX}-(?:{hues})(?:-\d{{2,3}})?(?:/\d+)?(?:$|[^\w-])"
    ))
    .unwrap();
    let found = offences(&files, &root, &raw);
    assert!(
        found.is_empty(),
        "ui primitives name raw palette colours — name a role instead (destructive, success, warning, info, muted, …):\n  {}",
        found.join("\n  ")
    );
}

/// Every primitive is on the gallery. `/dev/design-system` is where a
/// component's variants are seen together and where a theme change is judged;
/// a primitive that is not there is one nobody looks at. Each `Entry` on the
/// page carries `file="<name>.tsx"`; a primitive that is composed inside
/// another's entry (CardTitle inside Card, DialogFooter inside Dialog) counts
/// when the section imports it.
#[test]
fn every_ui_primitive_is_on_the_gallery() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut primitives = Vec::new();
    walk(&root.join("components/ui"), &mut primitives);
    let mut gallery = Vec::new();
    walk(&root.join("pages/dev/design-system"), &mut gallery);
    let shown: String = gallery
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect::<Vec<_>>()
        .join("\n");

    let missing: Vec<String> = primitives
        .iter()
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_owned))
        .filter(|name| !shown.contains(&format!("@/components/ui/{name}\"")))
        .collect();
    assert!(
        missing.is_empty(),
        "components/ui has primitives the gallery never imports — add an Entry under pages/dev/design-system/sections:\n  {}",
        missing.join("\n  ")
    );
}
