//! One file of a browser module folder, read once: compiled from TypeScript
//! to JavaScript, then parsed for what the bundler needs to know about it —
//! its imports and exports, the statements that go when files are joined,
//! every place a top-level name or an imported name is used, the lazy
//! `import()` calls and the `import.meta.glob` calls.
//!
//! Nothing here knows about other files: the analysis depends on the file's
//! bytes alone, so it is cached by them ([`super::analysis_cache`]).

use std::collections::{BTreeMap, HashSet};

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier,
    ModuleExportName, ObjectPropertyKind, PropertyKey, Statement,
};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};

use super::super::error::EngineError;

/// What a file is to the bundler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Kind {
    /// TypeScript or JavaScript, compiled and joined.
    Script,
    /// `import data from "./x.json"`: the parsed value is the default export.
    Json,
    /// `import text from "./x.glsl?raw"`: the file's text is the default export.
    Raw,
}

/// What an import binding takes from its module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Imported {
    Named(String),
    Default,
    Namespace,
}

/// One `import … from "spec"` statement (or a bare `import "spec"`).
#[derive(Debug, Clone)]
pub(super) struct ImportStmt {
    pub spec: String,
    /// `(local name, what it takes)`.
    pub bindings: Vec<(String, Imported)>,
}

/// What an exported name of this file is.
#[derive(Debug, Clone)]
pub(super) enum ExportTarget {
    /// A name declared or imported in this file (`export const x`, `export { x }`).
    Local(String),
    /// `export { x as y } from "spec"`, `export { default } from "spec"`.
    Reexport { spec: String, imported: Imported },
    /// `export * as ns from "spec"`.
    Namespace { spec: String },
}

/// A place in the compiled text where a name is written.
#[derive(Debug, Clone, Copy)]
pub(super) struct Site {
    pub start: u32,
    pub end: u32,
    /// Written as a shorthand (`{ x }`, `const { x } = …`): renaming must keep
    /// the property name, so it becomes `x: renamed`.
    pub shorthand: bool,
}

/// A change to the compiled text that does not depend on other files.
#[derive(Debug, Clone)]
pub(super) enum Edit {
    /// A statement that goes: an import, an `export … from`, an `export { … }`.
    Remove(u32, u32),
    /// The `export ` in front of a declaration.
    Strip(u32, u32),
    /// `export default <anonymous function | class | expression>`: the prefix
    /// becomes `const <name> = `, and a `;` follows at `end`.
    DefaultValue { start: u32, end: u32, stmt_end: u32 },
}

/// `import("spec")` with a fixed string: the string's span.
#[derive(Debug, Clone)]
pub(super) struct Dynamic {
    pub start: u32,
    pub end: u32,
    pub spec: String,
}

/// `import.meta.glob("pattern", { eager, import })`: the whole call's span.
#[derive(Debug, Clone)]
pub(super) struct Glob {
    pub start: u32,
    pub end: u32,
    pub patterns: Vec<String>,
    pub eager: bool,
    /// `import: "default"` or `import: "name"`: each value is that export, not the module.
    pub import: Option<String>,
}

/// Everything the bundler needs to know about one file.
#[derive(Debug, Clone)]
pub(super) struct Analysis {
    pub kind: Kind,
    /// The compiled JavaScript (for JSON and raw files, the expression that is its default export).
    pub js: String,
    pub imports: Vec<ImportStmt>,
    pub exports: BTreeMap<String, ExportTarget>,
    /// `export * from "spec"`.
    pub stars: Vec<String>,
    pub edits: Vec<Edit>,
    pub dynamic: Vec<Dynamic>,
    /// `new Worker(new URL("spec", import.meta.url), …)`: the string's span.
    pub workers: Vec<Dynamic>,
    pub globs: Vec<Glob>,
    /// Top-level declared names → where each is written (declaration included).
    pub top: BTreeMap<String, Vec<Site>>,
    /// Imported names → where each is used.
    pub import_refs: BTreeMap<String, Vec<Site>>,
    /// Every name the file declares in any scope, and every global it reads:
    /// a name chosen for the joined file must never be one of these.
    pub names: HashSet<String>,
}

/// The name a synthesized default export takes before the joined file
/// renames it.
pub(super) const DEFAULT_LOCAL: &str = "__default";

pub(super) fn analyze(path: &str, kind: Kind, source: &str) -> Result<Analysis, EngineError> {
    match kind {
        Kind::Json => {
            let value: serde_json::Value = serde_json::from_str(source)
                .map_err(|err| refuse(path, format!("is not valid JSON: {err}")))?;
            Ok(value_module(Kind::Json, serde_json::to_string(&value).unwrap_or_default()))
        }
        Kind::Raw => Ok(value_module(Kind::Raw, serde_json::to_string(source).unwrap_or_default())),
        Kind::Script => script(path, source),
    }
}

/// A JSON or raw-text file: one default export, nothing to parse.
fn value_module(kind: Kind, js: String) -> Analysis {
    let mut exports = BTreeMap::new();
    exports.insert("default".to_string(), ExportTarget::Local(DEFAULT_LOCAL.to_string()));
    let mut top = BTreeMap::new();
    top.insert(DEFAULT_LOCAL.to_string(), Vec::new());
    Analysis {
        kind,
        js,
        imports: Vec::new(),
        exports,
        stars: Vec::new(),
        edits: Vec::new(),
        dynamic: Vec::new(),
        workers: Vec::new(),
        globs: Vec::new(),
        top,
        import_refs: BTreeMap::new(),
        names: HashSet::new(),
    }
}

fn refuse(path: &str, why: String) -> EngineError {
    EngineError::new("RWE_MODULE_SOURCE", format!("'{path}' {why}"))
}

fn script(path: &str, source: &str) -> Result<Analysis, EngineError> {
    let js = if path.ends_with(".ts") || path.ends_with(".mts") {
        super::super::deno_worker::transpile_ts(source).map_err(|err| refuse(path, format!("does not compile: {}", err.message)))?
    } else {
        source.to_string()
    };
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, &js, SourceType::default().with_module(true)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        let first = parsed.errors.first().map(|e| e.message.to_string()).unwrap_or_else(|| "the parser gave up".to_string());
        return Err(refuse(path, format!("does not parse: {first}")));
    }
    let program = &parsed.program;

    let mut out = Analysis {
        kind: Kind::Script,
        js: String::new(),
        imports: Vec::new(),
        exports: BTreeMap::new(),
        stars: Vec::new(),
        edits: Vec::new(),
        dynamic: Vec::new(),
        workers: Vec::new(),
        globs: Vec::new(),
        top: BTreeMap::new(),
        import_refs: BTreeMap::new(),
        names: HashSet::new(),
    };

    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) => {
                out.edits.push(Edit::Remove(import.span.start, import.span.end));
                if import.import_kind.is_type() {
                    continue;
                }
                let mut bindings = Vec::new();
                for specifier in import.specifiers.iter().flatten() {
                    let local = specifier.local().name.to_string();
                    let imported = match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(named) => {
                            if named.import_kind.is_type() {
                                continue;
                            }
                            match named.imported.name().as_str() {
                                "default" => Imported::Default,
                                name => Imported::Named(name.to_string()),
                            }
                        }
                        ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => Imported::Default,
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => Imported::Namespace,
                    };
                    bindings.push((local, imported));
                }
                out.imports.push(ImportStmt { spec: import.source.value.to_string(), bindings });
            }
            Statement::ExportNamedDeclaration(export) => {
                if export.export_kind.is_type() {
                    out.edits.push(Edit::Remove(export.span.start, export.span.end));
                    continue;
                }
                if let Some(declaration) = &export.declaration {
                    out.edits.push(Edit::Strip(export.span.start, declaration.span().start));
                    for name in declared_names(declaration) {
                        out.exports.insert(name.clone(), ExportTarget::Local(name));
                    }
                    continue;
                }
                out.edits.push(Edit::Remove(export.span.start, export.span.end));
                for specifier in &export.specifiers {
                    if specifier.export_kind.is_type() {
                        continue;
                    }
                    let exported = export_name(&specifier.exported);
                    let local = export_name(&specifier.local);
                    let target = match &export.source {
                        Some(source) => ExportTarget::Reexport {
                            spec: source.value.to_string(),
                            imported: if local == "default" { Imported::Default } else { Imported::Named(local) },
                        },
                        None => ExportTarget::Local(local),
                    };
                    out.exports.insert(exported, target);
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let (named, body_start) = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        (function.id.as_ref().map(|id| id.name.to_string()), function.span.start)
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        (class.id.as_ref().map(|id| id.name.to_string()), class.span.start)
                    }
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => {
                        out.edits.push(Edit::Remove(export.span.start, export.span.end));
                        continue;
                    }
                    other => (None, other.span().start),
                };
                match named {
                    Some(name) => {
                        out.edits.push(Edit::Strip(export.span.start, body_start));
                        out.exports.insert("default".to_string(), ExportTarget::Local(name));
                    }
                    None => {
                        out.edits.push(Edit::DefaultValue { start: export.span.start, end: body_start, stmt_end: export.span.end });
                        out.exports.insert("default".to_string(), ExportTarget::Local(DEFAULT_LOCAL.to_string()));
                        out.top.insert(DEFAULT_LOCAL.to_string(), Vec::new());
                    }
                }
            }
            Statement::ExportAllDeclaration(export) => {
                out.edits.push(Edit::Remove(export.span.start, export.span.end));
                if export.export_kind.is_type() {
                    continue;
                }
                let spec = export.source.value.to_string();
                match &export.exported {
                    Some(name) => {
                        out.exports.insert(export_name(name), ExportTarget::Namespace { spec });
                    }
                    None => out.stars.push(spec),
                }
            }
            _ => {}
        }
    }

    // Where names are written: the semantic model resolves every reference to
    // the declaration it means, so a parameter or a property that happens to
    // share a name is never touched.
    let semantic = SemanticBuilder::new().build(program).semantic;
    let scoping = semantic.scoping();
    let root = scoping.root_scope_id();
    let mut shorthand_bindings = Visitor::default();
    shorthand_bindings.visit_program(program);
    for symbol in scoping.symbol_ids() {
        out.names.insert(scoping.symbol_name(symbol).to_string());
        if scoping.symbol_scope_id(symbol) != root {
            continue;
        }
        let name = scoping.symbol_name(symbol).to_string();
        let mut sites = Vec::new();
        for reference in scoping.get_resolved_references(symbol) {
            let node = reference.node_id();
            let span = semantic.nodes().get_node(node).kind().span();
            let shorthand = matches!(
                semantic.nodes().parent_kind(node),
                AstKind::ObjectProperty(property) if property.shorthand
            ) || matches!(semantic.nodes().parent_kind(node), AstKind::AssignmentTargetPropertyIdentifier(_));
            sites.push(Site { start: span.start, end: span.end, shorthand });
        }
        if scoping.symbol_flags(symbol).is_import() {
            out.import_refs.insert(name, sites);
        } else {
            let span = scoping.symbol_span(symbol);
            let shorthand = shorthand_bindings.shorthand.contains(&(span.start, span.end));
            sites.push(Site { start: span.start, end: span.end, shorthand });
            out.top.entry(name).or_default().extend(sites);
        }
    }
    for name in scoping.root_unresolved_references().keys() {
        out.names.insert(name.to_string());
    }

    out.dynamic = shorthand_bindings.dynamic;
    out.workers = shorthand_bindings.workers;
    out.globs = shorthand_bindings.globs;
    if let Some(bad) = shorthand_bindings.refused {
        return Err(refuse(path, bad));
    }
    out.js = js;
    Ok(out)
}

fn export_name(name: &ModuleExportName) -> String {
    name.name().to_string()
}

/// What one JavaScript module exports, read without running it: the names it
/// exports itself (`export * as ns` included), and the modules whose names
/// `export * from` passes on, left for the caller to read. Type-only exports
/// are not names a browser sees.
pub(crate) fn export_surface(source: &str) -> Result<(std::collections::BTreeSet<String>, Vec<String>), String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        let first = parsed.errors.first().map(|error| error.to_string()).unwrap_or_else(|| "it does not parse".to_string());
        return Err(first);
    }
    let (mut names, mut stars) = (std::collections::BTreeSet::new(), Vec::new());
    for statement in &parsed.program.body {
        match statement {
            Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => {
                if let Some(declaration) = &export.declaration {
                    names.extend(declared_names(declaration));
                }
                for specifier in export.specifiers.iter().filter(|specifier| !specifier.export_kind.is_type()) {
                    names.insert(export_name(&specifier.exported));
                }
            }
            Statement::ExportDefaultDeclaration(export)
                if !matches!(export.declaration, ExportDefaultDeclarationKind::TSInterfaceDeclaration(_)) =>
            {
                names.insert("default".to_string());
            }
            Statement::ExportAllDeclaration(export) if !export.export_kind.is_type() => match &export.exported {
                Some(name) => {
                    names.insert(export_name(name));
                }
                None => stars.push(export.source.value.to_string()),
            },
            _ => {}
        }
    }
    Ok((names, stars))
}

/// The names a declaration binds at the top level.
fn declared_names(declaration: &Declaration) -> Vec<String> {
    match declaration {
        Declaration::VariableDeclaration(variables) => {
            let mut names = Vec::new();
            for declarator in &variables.declarations {
                pattern_names(&declarator.id, &mut names);
            }
            names
        }
        Declaration::FunctionDeclaration(function) => function.id.iter().map(|id| id.name.to_string()).collect(),
        Declaration::ClassDeclaration(class) => class.id.iter().map(|id| id.name.to_string()).collect(),
        Declaration::TSEnumDeclaration(declaration) => vec![declaration.id.name.to_string()],
        _ => Vec::new(),
    }
}

fn pattern_names(pattern: &BindingPattern, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push(id.name.to_string()),
        BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                pattern_names(&property.value, out);
            }
            if let Some(rest) = &object.rest {
                pattern_names(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(array) => {
            for element in array.elements.iter().flatten() {
                pattern_names(element, out);
            }
            if let Some(rest) = &array.rest {
                pattern_names(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(assignment) => pattern_names(&assignment.left, out),
    }
}

/// Walks the whole file for what statements alone do not show.
#[derive(Default)]
struct Visitor {
    /// Spans of binding identifiers written as shorthand (`const { x } = …`).
    shorthand: HashSet<(u32, u32)>,
    dynamic: Vec<Dynamic>,
    workers: Vec<Dynamic>,
    globs: Vec<Glob>,
    refused: Option<String>,
}

impl<'a> Visit<'a> for Visitor {
    fn visit_binding_property(&mut self, property: &oxc_ast::ast::BindingProperty<'a>) {
        if property.shorthand {
            let target = match &property.value {
                BindingPattern::AssignmentPattern(assignment) => &assignment.left,
                other => other,
            };
            if let BindingPattern::BindingIdentifier(id) = target {
                self.shorthand.insert((id.span.start, id.span.end));
            }
        }
        oxc_ast_visit::walk::walk_binding_property(self, property);
    }

    fn visit_import_expression(&mut self, expression: &oxc_ast::ast::ImportExpression<'a>) {
        match &expression.source {
            Expression::StringLiteral(literal) => self.dynamic.push(Dynamic {
                start: literal.span.start,
                end: literal.span.end,
                spec: literal.value.to_string(),
            }),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() && template.quasis.len() == 1 => {
                self.dynamic.push(Dynamic {
                    start: template.span.start,
                    end: template.span.end,
                    spec: template.quasis[0].value.raw.to_string(),
                })
            }
            _ => {
                self.refused.get_or_insert_with(|| {
                    "loads a module whose name is computed; import() takes a fixed string, so what it loads is known when the folder is built".to_string()
                });
            }
        }
        oxc_ast_visit::walk::walk_import_expression(self, expression);
    }

    fn visit_new_expression(&mut self, new: &oxc_ast::ast::NewExpression<'a>) {
        // `new Worker(new URL("./x.worker.ts", import.meta.url), { type: "module" })`:
        // the standard way a module starts a worker, and Vite's.
        let is_worker = matches!(&new.callee, Expression::Identifier(id) if id.name == "Worker" || id.name == "SharedWorker");
        if is_worker
            && let Some(Argument::NewExpression(url)) = new.arguments.first()
            && matches!(&url.callee, Expression::Identifier(id) if id.name == "URL")
            && let Some(Argument::StringLiteral(literal)) = url.arguments.first()
            && matches!(url.arguments.get(1), Some(Argument::StaticMemberExpression(member))
                if member.property.name == "url" && matches!(&member.object, Expression::MetaProperty(meta) if meta.meta.name == "import"))
        {
            self.workers.push(Dynamic { start: literal.span.start, end: literal.span.end, spec: literal.value.to_string() });
        }
        oxc_ast_visit::walk::walk_new_expression(self, new);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if is_import_meta_glob(&call.callee) {
            match glob_call(call) {
                Ok(glob) => self.globs.push(glob),
                Err(why) => {
                    self.refused.get_or_insert(why);
                }
            }
            return;
        }
        oxc_ast_visit::walk::walk_call_expression(self, call);
    }
}

fn is_import_meta_glob(callee: &Expression) -> bool {
    let Expression::StaticMemberExpression(member) = callee else { return false };
    let Expression::MetaProperty(meta) = &member.object else { return false };
    meta.meta.name == "import" && meta.property.name == "meta" && member.property.name == "glob"
}

fn glob_call(call: &oxc_ast::ast::CallExpression) -> Result<Glob, String> {
    let fixed = "import.meta.glob takes fixed strings (\"./objects/*.ts\" or an array of them), so what it matches is known when the folder is built";
    let mut patterns = Vec::new();
    match call.arguments.first() {
        Some(Argument::StringLiteral(literal)) => patterns.push(literal.value.to_string()),
        Some(Argument::ArrayExpression(array)) => {
            for element in &array.elements {
                match element {
                    oxc_ast::ast::ArrayExpressionElement::StringLiteral(literal) => patterns.push(literal.value.to_string()),
                    _ => return Err(fixed.to_string()),
                }
            }
        }
        _ => return Err(fixed.to_string()),
    }
    let mut eager = false;
    let mut import = None;
    if let Some(Argument::ObjectExpression(object)) = call.arguments.get(1) {
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return Err("import.meta.glob options are written out ({ eager: true, import: \"default\" })".to_string());
            };
            let key = match &property.key {
                PropertyKey::StaticIdentifier(id) => id.name.to_string(),
                PropertyKey::StringLiteral(literal) => literal.value.to_string(),
                _ => return Err("import.meta.glob options are written out ({ eager: true, import: \"default\" })".to_string()),
            };
            match (key.as_str(), &property.value) {
                ("eager", Expression::BooleanLiteral(value)) => eager = value.value,
                ("import", Expression::StringLiteral(value)) => import = Some(value.value.to_string()),
                (other, _) => return Err(format!("import.meta.glob option '{other}' is not supported; it takes eager (true or false) and import (a fixed export name)")),
            }
        }
    }
    Ok(Glob { start: call.span.start, end: call.span.end, patterns, eager, import })
}
