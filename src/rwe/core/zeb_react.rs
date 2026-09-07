//! Public `zeb/react` imports bind to the already-installed engine in both V8
//! and the browser. This module is intrinsic, not an optional Hub library.

use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier};

use super::error::EngineError;

pub const SPECIFIER: &str = "zeb/react";

pub const EXPORTS: &[&str] = &[
    "h",
    "createElement",
    "Fragment",
    "ErrorBoundary",
    "jsx",
    "jsxs",
    "jsxDEV",
    "render",
    "hydrate",
    "renderToString",
    "createContext",
    "createPortal",
    "forwardRef",
    "memo",
    "useState",
    "useReducer",
    "useSyncExternalStore",
    "useRef",
    "useMemo",
    "useCallback",
    "useContext",
    "useId",
    "useEffect",
    "useLayoutEffect",
    "useImperativeHandle",
    // Zebflow's own helpers. They are not React, but they are imported the same
    // way for the same reason there is one door: a writer should never have to
    // remember which of two specifiers a name lives behind.
    "usePageState",
    "useRouter",
    "usePathname",
    "useSearchParams",
    "Link",
    "cx",
];

/// The helpers the platform installs as plain globals rather than onto the
/// engine object, so an import of one lowers to a different expression.
const PLATFORM_GLOBALS: &[&str] = &[
    "usePageState",
    "useRouter",
    "usePathname",
    "useSearchParams",
    "Link",
    "cx",
];

/// Preserve aliases, default imports and namespace imports when the bundler
/// removes module declarations. Explicit declarations also let the component
/// inliner isolate bindings from other files that import the same hook.
pub fn lower_import(import: &ImportDeclaration<'_>) -> Result<String, EngineError> {
    if import.import_kind.is_type() {
        return Ok(String::new());
    }
    let mut bindings = String::new();
    if let Some(specifiers) = &import.specifiers {
        for specifier in specifiers {
            let local = specifier.local().name.as_str();
            let value = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(named) => {
                    if named.import_kind.is_type() {
                        continue;
                    }
                    let imported = named.imported.name();
                    let name = imported.as_str();
                    if name == "default" {
                        "globalThis.__zebReact".to_string()
                    } else {
                        if !EXPORTS.contains(&name) {
                            return Err(EngineError::new(
                                "RWE_REACT_EXPORT",
                                format!("'{name}' is not exported by zeb/react"),
                            ));
                        }
                        // Quoted property access survives the inliner's local
                        // identifier prefixing without renaming the export.
                        if PLATFORM_GLOBALS.contains(&name) {
                            format!("globalThis.{name}")
                        } else {
                            format!("globalThis.__zebReact[\"{name}\"]")
                        }
                    }
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    // A namespace contains every named export, including the
                    // platform helpers installed outside the React engine.
                    // Keep default identity identical to a default import.
                    let helpers = PLATFORM_GLOBALS
                        .iter()
                        .map(|name| format!("\"{name}\": globalThis[\"{name}\"]"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "Object.freeze({{ ...globalThis.__zebReact, {helpers}, default: globalThis.__zebReact }})"
                    )
                }
                _ => "globalThis.__zebReact".to_string(),
            };
            bindings.push_str(&format!("const {local} = {value};\n"));
        }
    }
    Ok(bindings)
}
