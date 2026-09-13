//! `zeb/ui` is a source library: `.tsx` shipped with the platform that the
//! compiler inlines into a page the way it inlines an `@/` import. So the
//! page's Tailwind scan sees the component's classes, SSR renders it, and
//! hydration gets it — with no runtime bundle and no stylesheet of its own.
//!
//! These tests hold the library to the rules that make that work.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use zebflow::rwe::core::{CompileOptions, compile};
use zebflow::rwe::processors::tailwind::compiler::process_tailwind;

fn library_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("blessed/source-libraries/ui/0.1/src")
}

fn component_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(library_src())
        .expect("zeb/ui source dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "tsx"))
        .collect();
    out.sort();
    assert!(!out.is_empty(), "zeb/ui has no components");
    out
}

/// A component file imports from `zeb/react` and from `zeb/ui/<sibling>`, and
/// nothing else. That is what lets a cloned copy work unchanged in a project:
/// its imports still resolve after it leaves the library.
#[test]
fn a_component_imports_only_zeb_react_and_zeb_ui() {
    let import = Regex::new(r#"(?m)^import\s.*?from\s+["']([^"']+)["']"#).unwrap();
    let mut bad = Vec::new();
    for path in component_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        for m in import.captures_iter(&text) {
            let spec = &m[1];
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            // The one documented exception: the editor's engine is a runtime
            // library (`zeb/prosemirror`), too large to inline into every page.
            let allowed_runtime = file == "editor.tsx" && spec == "zeb/prosemirror";
            if spec != "zeb/react" && !spec.starts_with("zeb/ui/") && !allowed_runtime {
                bad.push(format!("{file}: {spec}"));
            }
        }
    }
    assert!(bad.is_empty(), "zeb/ui components may import only zeb/react and zeb/ui/*:\n  {}", bad.join("\n  "));
}

/// No raw palette, no retired token: the library is themed by role like the
/// studio primitives are. Same regexes as tests/rwe/theme_tokens.rs.
#[test]
fn a_component_names_roles_not_colours() {
    let prefix = r"(?:border-[trblxy]|ring-offset|bg|text|border|ring|fill|stroke|from|to|via|outline|divide|placeholder|caret|decoration|shadow)";
    let hues = "red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose|slate|gray|zinc|neutral|stone|white|black";
    let raw = Regex::new(&format!(
        r"(?:^|[^\w-])(?:[\w\[\]&>:.-]+:)*{prefix}-(?:{hues})(?:-\d{{2,3}})?(?:/\d+)?(?:$|[^\w-])"
    ))
    .unwrap();
    let mut bad = Vec::new();
    for path in component_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        for (i, line) in text.lines().enumerate() {
            for m in raw.find_iter(line) {
                bad.push(format!("{}:{}: {}", path.file_name().unwrap().to_string_lossy(), i + 1, m.as_str().trim()));
            }
        }
    }
    assert!(bad.is_empty(), "zeb/ui names raw colours — use a theme token:\n  {}", bad.join("\n  "));
}

/// Every class a component writes compiles to a rule. A shadcn class the
/// engine does not implement (`[&_svg]:size-4`, `has-[>svg]:px-3`) would be
/// silently dropped and the component would look subtly wrong; this names it.
#[test]
fn every_class_in_the_library_compiles() {
    let literal = Regex::new(r#""([^"\\]*)""#).unwrap();
    let looks_like_class = Regex::new(r"^[a-z!\[][\w\[\]().%#&>:/!,'=*-]*$").unwrap();
    let mut bad = Vec::new();
    for path in component_files() {
        let text = std::fs::read_to_string(&path).unwrap();
        for cap in literal.captures_iter(&text) {
            let s = &cap[1];
            let tokens: Vec<&str> = s.split_whitespace().collect();
            if tokens.len() < 2 || !tokens.iter().all(|t| looks_like_class.is_match(t)) {
                continue;
            }
            let html = format!("<div class=\"{s}\"></div>");
            let css = process_tailwind(&html, &HashSet::new());
            // A string is a class list when most of its tokens are known
            // utilities; a sentence — or a keyword table that happens to
            // contain `table` and `static` — is not.
            let known = tokens.iter().filter(|t| {
                let selector = format!(".{}", zebflow::rwe::processors::tailwind::compiler::escape_class_selector(t));
                css.contains(&selector)
            }).count();
            if known * 2 < tokens.len() {
                continue;
            }
            for t in tokens {
                // `group` and `peer` are markers other rules point at; they
                // have no rule of their own and are correct without one.
                if t == "group" || t == "peer" || t.starts_with("group/") || t.starts_with("peer/") {
                    continue;
                }
                let selector = format!(".{}", zebflow::rwe::processors::tailwind::compiler::escape_class_selector(t));
                if !css.contains(&selector) {
                    bad.push(format!("{}: `{t}`", path.file_name().unwrap().to_string_lossy()));
                }
            }
        }
    }
    bad.sort();
    bad.dedup();
    assert!(bad.is_empty(), "classes the engine does not compile:\n  {}", bad.join("\n  "));
}

/// The point of the design: a page that imports `zeb/ui/button` gets the
/// component inlined as source. No `zeb/ui` survives into the bundle, no
/// runtime library is detected, and the class strings are in the page for the
/// Tailwind scan to find.
#[test]
fn a_page_importing_zeb_ui_button_inlines_the_source() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let page = r#"import { Button } from "zeb/ui/button";
export default function Page() {
  return <main><Button variant="destructive" size="sm">Delete</Button></main>;
}
"#;
    let compiled = compile(
        page,
        CompileOptions { library_roots: roots, ..Default::default() },
    )
    .expect("page compiles");
    let server = &compiled.server_module_source;
    assert!(!server.contains("zeb/ui"), "the specifier must be resolved away:\n{server}");
    assert!(server.contains("bg-destructive"), "the component source must be inlined:\n{server}");
    assert!(
        !compiled.detected_zeb_libs.iter().any(|l| l.starts_with("zeb/ui")),
        "zeb/ui is source, not a runtime library: {:?}",
        compiled.detected_zeb_libs
    );
}

/// An import of a component the library does not have is refused at compile
/// time with the library's name in the message, not dropped.
#[test]
fn an_unknown_component_is_refused_by_name() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let page = r#"import { Wizard } from "zeb/ui/wizard";
export default function Page() { return <Wizard />; }
"#;
    let err = compile(page, CompileOptions { library_roots: roots, ..Default::default() })
        .expect_err("unknown component must be refused");
    let msg = format!("{err:?}");
    assert!(msg.contains("zeb/ui/wizard") && msg.contains("zeb/ui"), "{msg}");
}

/// Every component file compiles into a page through the real compiler —
/// parse errors, an import the rules refuse, a hook used without its import,
/// all surface here with the file's name.
#[test]
fn every_component_compiles_into_a_page() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let mut bad = Vec::new();
    for path in component_files() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let page = format!(
            "import Component from \"zeb/ui/{name}\";\nexport default function Page() {{ return <div data-has={{typeof Component}} />; }}\n"
        );
        if let Err(err) = compile(&page, CompileOptions { library_roots: roots.clone(), ..Default::default() }) {
            bad.push(format!("{name}: {} {}", err.code, err.message));
        }
    }
    assert!(bad.is_empty(), "zeb/ui components that do not compile:\n  {}", bad.join("\n  "));
}

/// Every component is on the gallery, in a `zeb-ui-*` section of
/// `/dev/design-system`, so its variants are seen and a theme change is judged.
#[test]
fn every_component_is_on_the_gallery() {
    let gallery = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates/pages/dev/design-system");
    let mut shown = String::new();
    fn walk(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                out.push_str(&t);
                out.push('\n');
            }
        }
    }
    walk(&gallery, &mut shown);
    let missing: Vec<String> = component_files()
        .iter()
        .map(|p| p.file_stem().unwrap().to_string_lossy().to_string())
        .filter(|name| !shown.contains(&format!("\"zeb/ui/{name}\"")))
        .collect();
    assert!(
        missing.is_empty(),
        "zeb/ui components the gallery never imports — add an Entry with file=\"zeb/ui/<name>\" under pages/dev/design-system/sections/zeb-ui-*.tsx:\n  {}",
        missing.join("\n  ")
    );
}

/// Every named export of a component file reaches the page. `export function
/// AlertTitle` inside a file with several exports must be a binding the page
/// can call — the first gallery render threw `AlertTitle is not defined`.
#[test]
fn every_named_export_of_a_component_reaches_the_page() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let export_re = Regex::new(r"(?m)^export (?:function|const) ([A-Za-z0-9_]+)").unwrap();
    let mut bad = Vec::new();
    for path in component_files() {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        let names: Vec<String> = export_re.captures_iter(&text).map(|c| c[1].to_string()).collect();
        if names.is_empty() {
            continue;
        }
        let page = format!(
            "import {{ {} }} from \"zeb/ui/{name}\";\nexport default function Page() {{ return <div>{}</div>; }}\n",
            names.join(", "),
            names.iter().map(|n| format!("{{typeof {n}}}")).collect::<Vec<_>>().join("")
        );
        match compile(&page, CompileOptions { library_roots: roots.clone(), ..Default::default() }) {
            Ok(compiled) => {
                for n in &names {
                    // A binding the bundle never declares is a ReferenceError at render.
                    let declared = Regex::new(&format!(r"(?m)^\s*(?:function|const|let|var)\s+{n}\b")).unwrap();
                    if !declared.is_match(&compiled.server_module_source) {
                        bad.push(format!("{name}: `{n}` is exported but not declared in the bundle"));
                    }
                }
            }
            Err(err) => bad.push(format!("{name}: {} {}", err.code, err.message)),
        }
    }
    assert!(bad.is_empty(), "exports that do not reach the page:\n  {}", bad.join("\n  "));
}

/// A page that reaches `zeb/ui` through one of its own components — the
/// gallery's shape, and any real page with a `components/` folder — inlines
/// the library too. The first gallery render threw `AlertTitle is not
/// defined` from exactly this path.
#[test]
fn a_component_that_imports_zeb_ui_inlines_it_transitively() {
    let root = std::env::temp_dir().join(format!("zeb-ui-transitive-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(
        root.join("components/notice.tsx"),
        "import { Alert, AlertTitle } from \"zeb/ui/alert\";\nexport default function Notice() { return <Alert><AlertTitle>hi</AlertTitle></Alert>; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("page.tsx"),
        "import Notice from \"@/components/notice\";\nexport default function Page() { return <main><Notice /></main>; }\n",
    )
    .unwrap();
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let source = std::fs::read_to_string(root.join("page.tsx")).unwrap();
    let compiled = compile(
        &source,
        CompileOptions {
            template_root: Some(root.display().to_string()),
            file_path: Some(root.join("page.tsx").display().to_string()),
            library_roots: roots,
            ..Default::default()
        },
    )
    .expect("page compiles");
    let server = &compiled.server_module_source;
    assert!(server.contains("function AlertTitle"), "AlertTitle must be inlined:\n{server}");
    assert!(!server.contains("zeb/ui/alert"), "the specifier must be resolved away:\n{server}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The materialized library survives the platform template root's debug
/// sweep: it lives under its own temp root, not under the template root.
#[test]
fn the_materialized_library_is_not_under_the_template_root() {
    let template_root = std::env::temp_dir().join("zebflow-platform");
    let service = zebflow::platform::services::library::LibraryService::from_embedded().expect("library service");
    let roots = service.source_roots();
    let ui = roots.get("zeb/ui").expect("zeb/ui is a source library");
    assert!(!ui.starts_with(&template_root), "{} is under the template root {}", ui.display(), template_root.display());
    assert!(ui.join("button.tsx").is_file(), "materialized button.tsx missing at {}", ui.display());
}

/// Only the import statement is rewritten. The same specifier inside a
/// caption or a code sample stays as written — the gallery prints
/// `import { Button } from "zeb/ui/button"` for a reader to copy, not the
/// temp path the compiler resolved it to.
#[test]
fn a_specifier_in_a_string_is_not_rewritten() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let page = "import { Button } from \"zeb/ui/button\";\nconst caption = 'import { Button } from \"zeb/ui/button\"';\nexport default function Page() { return <div title=\"zeb/ui/button\">{caption}<Button /></div>; }\n";
    let compiled = compile(page, CompileOptions { library_roots: roots, ..Default::default() }).expect("compiles");
    let server = &compiled.server_module_source;
    assert!(server.contains("const caption = 'import { Button } from \"zeb/ui/button\"'"), "caption rewritten:\n{server}");
    assert!(server.contains("title=\"zeb/ui/button\""), "attribute rewritten:\n{server}");
}

/// An aliased or default import of a component binds the name the page uses.
/// `import { Button as SaveButton }` and `import Btn from "zeb/ui/button"`
/// used to compile and then throw `SaveButton is not defined` at render.
#[test]
fn aliased_and_default_imports_bind_the_local_name() {
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let page = "import { Button as SaveButton } from \"zeb/ui/button\";\nimport Btn from \"zeb/ui/badge\";\nexport default function Page() { return <div><SaveButton>save</SaveButton><Btn>new</Btn></div>; }\n";
    let compiled = compile(page, CompileOptions { library_roots: roots, ..Default::default() }).expect("compiles");
    let server = &compiled.server_module_source;
    assert!(server.contains("const SaveButton = Button;"), "alias not bound:\n{server}");
    assert!(server.contains("const Btn = Badge;"), "default import not bound:\n{server}");
}

/// Two modules exporting the same name cannot share a page's flat bundle.
/// The compiler says so, naming both files — v8 used to say `Identifier
/// 'Button' has already been declared` at render, naming a temp path.
#[test]
fn two_modules_exporting_one_name_are_refused_by_name() {
    let root = std::env::temp_dir().join(format!("zeb-ui-collision-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("shared/ui")).unwrap();
    std::fs::write(root.join("shared/ui/button.tsx"), "export function Button() { return <button>mine</button>; }\nexport default Button;\n").unwrap();
    std::fs::write(
        root.join("page.tsx"),
        "import { Button } from \"@/shared/ui/button\";\nimport { Badge } from \"zeb/ui/badge\";\nimport { AlertDialog } from \"zeb/ui/alert-dialog\";\nexport default function Page() { return <div><Button /><Badge /><AlertDialog /></div>; }\n",
    )
    .unwrap();
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let source = std::fs::read_to_string(root.join("page.tsx")).unwrap();
    let err = compile(
        &source,
        CompileOptions {
            template_root: Some(root.display().to_string()),
            file_path: Some(root.join("page.tsx").display().to_string()),
            library_roots: roots,
            ..Default::default()
        },
    )
    .expect_err("a cloned Button next to zeb/ui's (reached through alert-dialog) must be refused");
    assert_eq!(err.code, "RWE_BUNDLE_NAME_COLLISION", "{err:?}");
    assert!(err.message.contains("'Button'") && err.message.contains("shared/ui/button.tsx"), "{}", err.message);
    let _ = std::fs::remove_dir_all(&root);
}

/// An unknown component imported by a *component* (not the page) is refused
/// by name too, not silently stripped as if it were a runtime library.
#[test]
fn an_unknown_component_reached_transitively_is_refused_by_name() {
    let root = std::env::temp_dir().join(format!("zeb-ui-transitive-unknown-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("components")).unwrap();
    std::fs::write(root.join("components/wiz.tsx"), "import { Wizard } from \"zeb/ui/wizard\";\nexport default function Wiz() { return <Wizard />; }\n").unwrap();
    std::fs::write(root.join("page.tsx"), "import Wiz from \"@/components/wiz\";\nexport default function Page() { return <Wiz />; }\n").unwrap();
    let mut roots = BTreeMap::new();
    roots.insert("zeb/ui".to_string(), library_src().display().to_string());
    let source = std::fs::read_to_string(root.join("page.tsx")).unwrap();
    let err = compile(&source, CompileOptions { template_root: Some(root.display().to_string()), file_path: Some(root.join("page.tsx").display().to_string()), library_roots: roots, ..Default::default() })
        .expect_err("must be refused");
    assert!(err.message.contains("zeb/ui/wizard"), "{err:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A side override (`rounded-l-none`, `border-l-0`) must win over the
/// shorthand it refines (`rounded-md`, `border`) whichever way a template
/// writes them — Tailwind orders the stylesheet by specificity of intent,
/// not by first sighting. ButtonGroup's squared inner corners depend on it.
#[test]
fn side_overrides_win_over_their_shorthand() {
    for html in ["<div class=\"rounded-md rounded-l-none border border-l-0\"></div>", "<div class=\"rounded-l-none rounded-md border-l-0 border\"></div>"] {
        let css = process_tailwind(html, &HashSet::new());
        let md = css.find(".rounded-md{").expect("rounded-md rule");
        let l = css.find(".rounded-l-none{").expect("rounded-l-none rule");
        assert!(l > md, "rounded-l-none must come after rounded-md:\n{css}");
        let b = css.find(".border{").expect("border rule");
        let bl = css.find(".border-l-0{").expect("border-l-0 rule");
        assert!(bl > b, "border-l-0 must come after border:\n{css}");
    }
}
