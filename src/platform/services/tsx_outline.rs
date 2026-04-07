//! OXC-based AST outline extractor for TSX/TS/JS template files.
//!
//! Produces a compact symbol outline (imports, functions, classes, constants,
//! types, interfaces) with line numbers — enabling agents to understand file
//! structure without reading the full source.

use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Kind of top-level symbol found in the outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Import,
    Function,
    Class,
    Const,
    Type,
    Interface,
}

impl SymbolKind {
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Import => "import",
            SymbolKind::Function => "fn",
            SymbolKind::Class => "class",
            SymbolKind::Const => "const",
            SymbolKind::Type => "type",
            SymbolKind::Interface => "interface",
        }
    }
}

/// A single symbol in the outline.
#[derive(Debug, Clone)]
pub struct OutlineSymbol {
    pub kind: SymbolKind,
    pub name: String,
    /// 1-based start line.
    pub line: u32,
    /// 1-based end line.
    pub end_line: u32,
    /// Extra detail (e.g. import source path).
    pub detail: Option<String>,
    pub is_default: bool,
    pub is_exported: bool,
}

/// Result of outline extraction.
#[derive(Debug)]
pub struct OutlineResult {
    pub symbols: Vec<OutlineSymbol>,
    pub line_count: u32,
    pub parse_errors: Vec<String>,
}

/// Extract a structural outline from a TSX/TS/JS source string.
pub fn extract_outline(source: &str, _file_path: Option<&str>) -> OutlineResult {
    let line_count = source.lines().count() as u32;
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();

    let parse_errors: Vec<String> = parsed
        .errors
        .iter()
        .take(5)
        .map(|e| e.to_string())
        .collect();

    if parsed.panicked {
        return OutlineResult {
            symbols: Vec::new(),
            line_count,
            parse_errors,
        };
    }

    let mut symbols: Vec<OutlineSymbol> = Vec::new();

    for stmt in &parsed.program.body {
        match stmt {
            // ── Import declarations ──────────────────────────────────────────
            Statement::ImportDeclaration(imp) => {
                let source_val = imp.source.value.as_str();
                let specifiers: Vec<String> = imp
                    .specifiers
                    .as_ref()
                    .map(|specs| {
                        specs
                            .iter()
                            .map(|s| match s {
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(
                                    d,
                                ) => d.local.name.as_str().to_string(),
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(n) => {
                                    n.local.name.as_str().to_string()
                                }
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                                    ns,
                                ) => format!("* as {}", ns.local.name.as_str()),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let name = if specifiers.is_empty() {
                    format!("\"{}\"", source_val)
                } else {
                    format!("{{ {} }}", specifiers.join(", "))
                };
                let line = line_of(source, imp.span.start);
                let end_line = line_of(source, imp.span.end.saturating_sub(1));
                symbols.push(OutlineSymbol {
                    kind: SymbolKind::Import,
                    name,
                    line,
                    end_line,
                    detail: Some(source_val.to_string()),
                    is_default: false,
                    is_exported: false,
                });
            }

            // ── export { ... } / export const / export function ──────────────
            Statement::ExportNamedDeclaration(ed) => {
                if let Some(decl) = &ed.declaration {
                    for sym in extract_declaration(source, decl) {
                        symbols.push(OutlineSymbol {
                            is_exported: true,
                            ..sym
                        });
                    }
                }
                // Re-exports: export { X } from "..."
                if !ed.specifiers.is_empty() && ed.source.is_some() {
                    let src = ed
                        .source
                        .as_ref()
                        .map(|s| s.value.as_str())
                        .unwrap_or("?");
                    let names: Vec<String> = ed
                        .specifiers
                        .iter()
                        .map(|s| match &s.exported {
                            oxc_ast::ast::ModuleExportName::IdentifierName(n) => {
                                n.name.as_str().to_string()
                            }
                            oxc_ast::ast::ModuleExportName::IdentifierReference(r) => {
                                r.name.as_str().to_string()
                            }
                            oxc_ast::ast::ModuleExportName::StringLiteral(s) => {
                                s.value.as_str().to_string()
                            }
                        })
                        .collect();
                    let line = line_of(source, ed.span.start);
                    let end_line = line_of(source, ed.span.end.saturating_sub(1));
                    symbols.push(OutlineSymbol {
                        kind: SymbolKind::Import,
                        name: format!("{{ {} }}", names.join(", ")),
                        line,
                        end_line,
                        detail: Some(format!("re-export from \"{}\"", src)),
                        is_default: false,
                        is_exported: true,
                    });
                }
            }

            // ── export default ───────────────────────────────────────────────
            Statement::ExportDefaultDeclaration(edd) => {
                use oxc_ast::ast::ExportDefaultDeclarationKind;
                let line = line_of(source, edd.span.start);
                let end_line = line_of(source, edd.span.end.saturating_sub(1));
                match &edd.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        let name = f
                            .id
                            .as_ref()
                            .map(|id| id.name.as_str().to_string())
                            .unwrap_or_else(|| "(anonymous)".to_string());
                        symbols.push(OutlineSymbol {
                            kind: SymbolKind::Function,
                            name,
                            line,
                            end_line,
                            detail: None,
                            is_default: true,
                            is_exported: true,
                        });
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                        let name = c
                            .id
                            .as_ref()
                            .map(|id| id.name.as_str().to_string())
                            .unwrap_or_else(|| "(anonymous)".to_string());
                        symbols.push(OutlineSymbol {
                            kind: SymbolKind::Class,
                            name,
                            line,
                            end_line,
                            detail: None,
                            is_default: true,
                            is_exported: true,
                        });
                    }
                    _ => {
                        symbols.push(OutlineSymbol {
                            kind: SymbolKind::Const,
                            name: "default".to_string(),
                            line,
                            end_line,
                            detail: None,
                            is_default: true,
                            is_exported: true,
                        });
                    }
                }
            }

            // ── Top-level function / class / const (non-exported) ────────────
            Statement::FunctionDeclaration(f) => {
                if let Some(id) = &f.id {
                    let line = line_of(source, f.span.start);
                    let end_line = line_of(source, f.span.end.saturating_sub(1));
                    symbols.push(OutlineSymbol {
                        kind: SymbolKind::Function,
                        name: id.name.as_str().to_string(),
                        line,
                        end_line,
                        detail: None,
                        is_default: false,
                        is_exported: false,
                    });
                }
            }
            Statement::ClassDeclaration(c) => {
                if let Some(id) = &c.id {
                    let line = line_of(source, c.span.start);
                    let end_line = line_of(source, c.span.end.saturating_sub(1));
                    symbols.push(OutlineSymbol {
                        kind: SymbolKind::Class,
                        name: id.name.as_str().to_string(),
                        line,
                        end_line,
                        detail: None,
                        is_default: false,
                        is_exported: false,
                    });
                }
            }
            Statement::VariableDeclaration(vd) => {
                for d in &vd.declarations {
                    if let Some(name) = binding_ident_name(&d.id) {
                        let line = line_of(source, d.span.start);
                        let end_line = line_of(source, d.span.end.saturating_sub(1));
                        symbols.push(OutlineSymbol {
                            kind: SymbolKind::Const,
                            name,
                            line,
                            end_line,
                            detail: None,
                            is_default: false,
                            is_exported: false,
                        });
                    }
                }
            }
            Statement::TSTypeAliasDeclaration(t) => {
                let line = line_of(source, t.span.start);
                let end_line = line_of(source, t.span.end.saturating_sub(1));
                symbols.push(OutlineSymbol {
                    kind: SymbolKind::Type,
                    name: t.id.name.as_str().to_string(),
                    line,
                    end_line,
                    detail: None,
                    is_default: false,
                    is_exported: false,
                });
            }
            Statement::TSInterfaceDeclaration(iface) => {
                let line = line_of(source, iface.span.start);
                let end_line = line_of(source, iface.span.end.saturating_sub(1));
                symbols.push(OutlineSymbol {
                    kind: SymbolKind::Interface,
                    name: iface.id.name.as_str().to_string(),
                    line,
                    end_line,
                    detail: None,
                    is_default: false,
                    is_exported: false,
                });
            }

            _ => {}
        }
    }

    OutlineResult {
        symbols,
        line_count,
        parse_errors,
    }
}

/// Extract import source paths from a source file.
/// Returns a list of string literal values from all `import ... from "..."` statements.
pub fn extract_import_sources(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }

    let mut sources = Vec::new();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(imp) = stmt {
            sources.push(imp.source.value.as_str().to_string());
        }
        // Also catch re-exports: export { X } from "..."
        if let Statement::ExportNamedDeclaration(ed) = stmt {
            if let Some(src) = &ed.source {
                sources.push(src.value.as_str().to_string());
            }
        }
    }
    sources
}

/// Format an `OutlineResult` as human-readable text.
pub fn format_outline(rel_path: &str, result: &OutlineResult) -> String {
    let mut out = format!("# {} ({} lines)\n", rel_path, result.line_count);

    if !result.parse_errors.is_empty() {
        out.push_str("\nPARSE ERRORS\n");
        for e in &result.parse_errors {
            out.push_str(&format!("  {}\n", e));
        }
    }

    // Separate imports from other symbols.
    let imports: Vec<&OutlineSymbol> = result
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Import)
        .collect();
    let others: Vec<&OutlineSymbol> = result
        .symbols
        .iter()
        .filter(|s| s.kind != SymbolKind::Import)
        .collect();

    if !imports.is_empty() {
        out.push_str("\nIMPORTS\n");
        for s in &imports {
            let src = s
                .detail
                .as_deref()
                .map(|d| format!(" from \"{}\"", d))
                .unwrap_or_default();
            out.push_str(&format!("  {:>3}  {}{}\n", s.line, s.name, src));
        }
    }

    if !others.is_empty() {
        out.push_str("\nSYMBOLS\n");
        for s in &others {
            let range = if s.line == s.end_line {
                format!("{:>3}    ", s.line)
            } else {
                format!("{:>3}-{:<3}", s.line, s.end_line)
            };
            let mut tags = Vec::new();
            if s.is_exported {
                tags.push("exported");
            }
            if s.is_default {
                tags.push("default");
            }
            let tag_str = if tags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", tags.join(", "))
            };
            out.push_str(&format!(
                "  {} {} {}{}\n",
                range,
                s.kind.label(),
                s.name,
                tag_str,
            ));
        }
    }

    out
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Convert a byte offset to a 1-based line number.
fn line_of(source: &str, byte_offset: u32) -> u32 {
    let offset = (byte_offset as usize).min(source.len());
    source[..offset].matches('\n').count() as u32 + 1
}

/// Extract symbols from a `Declaration` node (used for both exported and non-exported).
fn extract_declaration(
    source: &str,
    decl: &oxc_ast::ast::Declaration,
) -> Vec<OutlineSymbol> {
    use oxc_ast::ast::Declaration;
    let mut syms = Vec::new();

    match decl {
        Declaration::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let line = line_of(source, f.span.start);
                let end_line = line_of(source, f.span.end.saturating_sub(1));
                syms.push(OutlineSymbol {
                    kind: SymbolKind::Function,
                    name: id.name.as_str().to_string(),
                    line,
                    end_line,
                    detail: None,
                    is_default: false,
                    is_exported: false,
                });
            }
        }
        Declaration::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let line = line_of(source, c.span.start);
                let end_line = line_of(source, c.span.end.saturating_sub(1));
                syms.push(OutlineSymbol {
                    kind: SymbolKind::Class,
                    name: id.name.as_str().to_string(),
                    line,
                    end_line,
                    detail: None,
                    is_default: false,
                    is_exported: false,
                });
            }
        }
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(name) = binding_ident_name(&d.id) {
                    let line = line_of(source, d.span.start);
                    let end_line = line_of(source, d.span.end.saturating_sub(1));
                    syms.push(OutlineSymbol {
                        kind: SymbolKind::Const,
                        name,
                        line,
                        end_line,
                        detail: None,
                        is_default: false,
                        is_exported: false,
                    });
                }
            }
        }
        Declaration::TSTypeAliasDeclaration(t) => {
            let line = line_of(source, t.span.start);
            let end_line = line_of(source, t.span.end.saturating_sub(1));
            syms.push(OutlineSymbol {
                kind: SymbolKind::Type,
                name: t.id.name.as_str().to_string(),
                line,
                end_line,
                detail: None,
                is_default: false,
                is_exported: false,
            });
        }
        Declaration::TSInterfaceDeclaration(iface) => {
            let line = line_of(source, iface.span.start);
            let end_line = line_of(source, iface.span.end.saturating_sub(1));
            syms.push(OutlineSymbol {
                kind: SymbolKind::Interface,
                name: iface.id.name.as_str().to_string(),
                line,
                end_line,
                detail: None,
                is_default: false,
                is_exported: false,
            });
        }
        _ => {}
    }

    syms
}

/// Get the simple identifier name from a binding pattern, or `None` for destructuring patterns.
fn binding_ident_name(pattern: &oxc_ast::ast::BindingPattern) -> Option<String> {
    if let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = pattern {
        Some(id.name.as_str().to_string())
    } else {
        None
    }
}
