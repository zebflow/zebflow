//! Runtime `zeb/*` library imports bind to the library that was imported.
//!
//! A runtime library (`zeb/threejs`, `zeb/d3`, …) is a browser bundle the page
//! loads before its own module runs. The loader files each one under a single
//! registry, `globalThis.__zebLibs["zeb/<name>"]`, and the compiler turns every
//! import of it into plain declarations that read that entry. A library's
//! exports are never installed as globals: two libraries may export the same
//! name, and a library may export a name the browser already owns.
//!
//! The server registers its placeholders under the same keys
//! (`runtime/zeb_ssr_libs.js`), so the same lowering serves both sides.
//! `zeb/react` has its own lowering ([`super::zeb_react`]) and `zeb/ui` is a
//! source library inlined at compile time; neither comes through here.

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier};

/// The registry every runtime library is filed under.
pub const REGISTRY: &str = "globalThis.__zebLibs";

/// Declarations for what one `import … from "zeb/<name>"` binds.
///
/// Property access is quoted so the inliner's identifier prefixing, which
/// renames a component file's module-level names, cannot rename an export.
/// Type-only imports bind nothing, and a bare `import "zeb/x"` binds nothing
/// either (the library is still loaded).
pub fn lower_import(import: &ImportDeclaration<'_>) -> String {
    if import.import_kind.is_type() {
        return String::new();
    }
    let library = quote(import.source.value.as_str());
    let mut bindings = String::new();
    let Some(specifiers) = &import.specifiers else {
        return bindings;
    };
    for specifier in specifiers {
        let local = specifier.local().name.as_str();
        let value = match specifier {
            ImportDeclarationSpecifier::ImportSpecifier(named) => {
                if named.import_kind.is_type() {
                    continue;
                }
                let imported = named.imported.name();
                format!("{REGISTRY}[{library}][{}]", quote(imported.as_str()))
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                format!("{REGISTRY}[{library}][\"default\"]")
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                format!("{REGISTRY}[{library}]")
            }
        };
        bindings.push_str(&format!("const {local} = {value};\n"));
    }
    bindings
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    fn lower(source: &str) -> String {
        let alloc = Allocator::default();
        let source_type = SourceType::default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true);
        let parsed = Parser::new(&alloc, source, source_type).parse();
        parsed
            .program
            .body
            .iter()
            .filter_map(|stmt| match stmt {
                Statement::ImportDeclaration(import) => Some(lower_import(import)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_named_import_reads_the_library_entry() {
        assert_eq!(
            lower(r#"import { mountThreeScene, REVISION } from "zeb/threejs";"#),
            "const mountThreeScene = globalThis.__zebLibs[\"zeb/threejs\"][\"mountThreeScene\"];\n\
             const REVISION = globalThis.__zebLibs[\"zeb/threejs\"][\"REVISION\"];\n"
        );
    }

    #[test]
    fn a_renamed_import_binds_the_local_name_to_the_export() {
        assert_eq!(
            lower(r#"import { Camera as ThreeCamera } from "zeb/threejs";"#),
            "const ThreeCamera = globalThis.__zebLibs[\"zeb/threejs\"][\"Camera\"];\n"
        );
    }

    #[test]
    fn a_namespace_import_binds_the_entry_itself() {
        assert_eq!(
            lower(r#"import * as d3lib from "zeb/d3";"#),
            "const d3lib = globalThis.__zebLibs[\"zeb/d3\"];\n"
        );
    }

    #[test]
    fn a_default_import_binds_the_default_export() {
        assert_eq!(
            lower(r#"import Everything from "zeb/pdf";"#),
            "const Everything = globalThis.__zebLibs[\"zeb/pdf\"][\"default\"];\n"
        );
        assert_eq!(
            lower(r#"import Whole, { createDocument } from "zeb/pdf";"#),
            "const Whole = globalThis.__zebLibs[\"zeb/pdf\"][\"default\"];\n\
             const createDocument = globalThis.__zebLibs[\"zeb/pdf\"][\"createDocument\"];\n"
        );
    }

    #[test]
    fn type_only_imports_bind_nothing() {
        assert_eq!(lower(r#"import type { Scene } from "zeb/threejs";"#), "");
        assert_eq!(
            lower(r#"import { type Scene, Mesh } from "zeb/threejs";"#),
            "const Mesh = globalThis.__zebLibs[\"zeb/threejs\"][\"Mesh\"];\n"
        );
    }

    #[test]
    fn a_bare_import_binds_nothing() {
        assert_eq!(lower(r#"import "zeb/potoru";"#), "");
    }

    #[test]
    fn two_libraries_exporting_one_name_stay_apart() {
        let lowered = lower(
            "import { Camera } from \"zeb/threejs\";\n\
             import { Camera as WorldCamera } from \"zeb/world\";",
        );
        assert!(lowered.contains("const Camera = globalThis.__zebLibs[\"zeb/threejs\"][\"Camera\"];"));
        assert!(lowered.contains(
            "const WorldCamera = globalThis.__zebLibs[\"zeb/world\"][\"Camera\"];"
        ));
    }
}
