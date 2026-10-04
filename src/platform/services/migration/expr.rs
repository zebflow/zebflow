//! The payload references inside 0.10 expressions, script bodies and pages,
//! found by the JavaScript parser rather than by a pattern: a string, a
//! comment or a shadowing parameter named `input` is never a reference.
//!
//! A reference is a free `input` / `$input` (or `$nodes.<id>`, a script's
//! `ctx.nodes.<id>`, a page's props) followed by a static path. The rewriter
//! replaces its *head* — the root and the first key — and leaves the rest of
//! the path and every other byte as written.

use std::ops::Range;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_semantic::{NodeId, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};

/// What a reference starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Root {
    /// The payload the node (or page) receives: `input`, `$input`, a
    /// page's props.
    Input,
    /// The payload node `<id>` answered: `$nodes.<id>`, `ctx.nodes.<id>`.
    Nodes(String),
}

/// One reference: its root, the static keys after it, and the byte range a
/// rewrite replaces (from the root to the end of the first key).
#[derive(Debug, Clone)]
pub struct Reference {
    pub root: Root,
    /// The root as written (`input`, `$input`, `props`); for a `Nodes`
    /// root, what comes before the id (`$nodes`, `ctx.nodes`).
    pub root_text: String,
    /// The static keys after the root (after the id for `Nodes`).
    pub path: Vec<String>,
    /// From the root's first byte to the end of the last static key: what
    /// a rewrite replaces.
    pub head: Range<usize>,
    /// The chain used optional access (`input?.rows`); the rewrite keeps it.
    pub optional: bool,
}

/// A root used without a static first key: `input` alone, `input[k]`,
/// `{ ...input }`. Nothing can say which key it reads.
#[derive(Debug, Clone)]
pub struct BareUse {
    pub text: String,
    pub at: Range<usize>,
    /// The use is the root alone (`input`), not a computed key.
    pub whole: bool,
    /// The root is the payload (`input`), not `$nodes`.
    pub input: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Scan {
    pub references: Vec<Reference>,
    pub bare: Vec<BareUse>,
    /// Keys a page destructures from its props (`function Page({ rows })`).
    pub destructured: Vec<String>,
}

/// How a text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    /// One JavaScript expression (`{{ … }}`, `logic.if --expr`).
    Expression,
    /// A script body (`n.script`'s source): `return` is allowed, `input` is
    /// its parameter and `ctx.nodes` holds the answers of earlier nodes.
    FunctionBody,
    /// A TSX page: the default export's first parameter, and the `input`
    /// and `ctx` globals, are the payload.
    Page,
    /// A component a page imports: only the `input` and `ctx` globals are
    /// the page's payload (its own parameters are its props).
    Component,
}

const EXPRESSION_PREFIX: &str = "(\n";
const EXPRESSION_SUFFIX: &str = "\n);";
const BODY_PREFIX: &str = "(async function __zf_migrate(input, ctx) {\n";
const BODY_SUFFIX: &str = "\n});";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartKind {
    Input,
    Nodes,
    /// A script's `ctx`: `ctx.nodes.<id>` is a `Nodes` root.
    Ctx,
}

/// Every payload reference in `source`, or why it cannot be read.
pub fn scan(source: &str, wrap: Wrap) -> Result<Scan, String> {
    let (prefix, suffix) = match wrap {
        Wrap::Expression => (EXPRESSION_PREFIX, EXPRESSION_SUFFIX),
        Wrap::FunctionBody => (BODY_PREFIX, BODY_SUFFIX),
        Wrap::Page | Wrap::Component => ("", ""),
    };
    let text = format!("{prefix}{source}{suffix}");
    let allocator = Allocator::default();
    let source_type = match wrap {
        Wrap::Page | Wrap::Component => SourceType::default().with_module(true).with_jsx(true).with_typescript(true),
        _ => SourceType::default().with_module(false),
    };
    let parsed = Parser::new(&allocator, &text, source_type).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        let message = parsed
            .errors
            .first()
            .map(|e| e.message.to_string())
            .unwrap_or_else(|| "the parser gave up".to_string());
        return Err(format!("does not parse: {message}"));
    }
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let offset = prefix.len();
    let mut out = Scan::default();

    let mut starts: Vec<(NodeId, StartKind)> = Vec::new();
    let free = |names: &[(&str, StartKind)], starts: &mut Vec<(NodeId, StartKind)>| {
        for (name, ids) in scoping.root_unresolved_references() {
            let name: &str = name;
            if let Some((_, kind)) = names.iter().find(|(n, _)| *n == name) {
                for id in ids {
                    starts.push((scoping.get_reference(*id).node_id(), *kind));
                }
            }
        }
    };
    match wrap {
        // An expression runs inside the same `function(input, n, ctx)` a
        // script does: `ctx.nodes.<id>` is `$nodes.<id>`.
        Wrap::Expression => free(
            &[("input", StartKind::Input), ("$input", StartKind::Input), ("$nodes", StartKind::Nodes), ("ctx", StartKind::Ctx)],
            &mut starts,
        ),
        Wrap::FunctionBody => {
            // The wrapper's own `input` and `ctx` parameters.
            let input_at = BODY_PREFIX.find("(input").map(|i| i + 1).unwrap_or(usize::MAX);
            let ctx_at = BODY_PREFIX.find(", ctx").map(|i| i + 2).unwrap_or(usize::MAX);
            for (param, kind, at) in [("input", StartKind::Input, input_at), ("ctx", StartKind::Ctx, ctx_at)] {
                for symbol in scoping.symbol_ids() {
                    if scoping.symbol_name(symbol) == param && scoping.symbol_span(symbol).start as usize == at {
                        for reference in scoping.get_resolved_references(symbol) {
                            starts.push((reference.node_id(), kind));
                        }
                    }
                }
            }
            free(&[("$input", StartKind::Input), ("$nodes", StartKind::Nodes)], &mut starts);
        }
        Wrap::Page => {
            let (symbols, destructured) = page_props(&semantic);
            for symbol in symbols {
                for reference in scoping.get_resolved_references(symbol) {
                    starts.push((reference.node_id(), StartKind::Input));
                }
            }
            out.destructured = destructured;
            free(&[("input", StartKind::Input), ("ctx", StartKind::Input)], &mut starts);
        }
        Wrap::Component => free(&[("input", StartKind::Input), ("ctx", StartKind::Input)], &mut starts),
    }

    for (node_id, kind) in starts {
        let start_span = nodes.get_node(node_id).kind().span();
        // (key, span of the key text, computed)
        let mut keys: Vec<(String, Span, bool)> = Vec::new();
        let mut optional = false;
        let mut dynamic = false;
        let mut current = node_id;
        let mut current_span = start_span;
        loop {
            let parent = nodes.parent_node(current);
            match parent.kind() {
                AstKind::StaticMemberExpression(member) if member.object.span() == current_span => {
                    keys.push((member.property.name.to_string(), member.property.span, false));
                    optional |= member.optional;
                    current_span = member.span;
                    current = parent.id();
                }
                AstKind::ComputedMemberExpression(member) if member.object.span() == current_span => {
                    match &member.expression {
                        Expression::StringLiteral(lit) => {
                            keys.push((lit.value.to_string(), member.span, true));
                        }
                        Expression::NumericLiteral(lit) => {
                            keys.push((lit.value.to_string(), member.span, true));
                        }
                        _ => {
                            dynamic = true;
                            break;
                        }
                    }
                    optional |= member.optional;
                    current_span = member.span;
                    current = parent.id();
                }
                _ => break,
            }
        }
        let chain_end = keys.last().map(|k| k.1.end as usize).unwrap_or(start_span.end as usize);
        let root_start = start_span.start as usize;
        let end_of = |i: usize| keys[i].1.end as usize;
        let is_input = kind == StartKind::Input;
        let bare = |out: &mut Scan, whole: bool| {
            out.bare.push(BareUse {
                text: slice(&text, root_start, current_span.end as usize),
                at: shift(root_start, current_span.end as usize, offset),
                whole,
                input: is_input,
            });
        };
        match kind {
            StartKind::Input => match keys.first() {
                Some((_, _, false)) => out.references.push(Reference {
                    root: Root::Input,
                    root_text: slice(&text, root_start, start_span.end as usize),
                    path: keys.iter().map(|(k, _, _)| k.clone()).collect(),
                    head: shift(root_start, chain_end, offset),
                    optional,
                }),
                Some(_) => bare(&mut out, false),
                None => bare(&mut out, !dynamic),
            },
            StartKind::Nodes | StartKind::Ctx => {
                // `$nodes.<id>…` or `ctx.nodes.<id>…`.
                let id_at = if kind == StartKind::Ctx {
                    if keys.first().map(|k| k.0.as_str()) != Some("nodes") {
                        continue;
                    }
                    1
                } else {
                    0
                };
                let Some((id, _, _)) = keys.get(id_at) else {
                    bare(&mut out, false);
                    continue;
                };
                let base_end = if id_at == 0 { start_span.end as usize } else { end_of(0) };
                let path: Vec<String> = keys[id_at + 1..].iter().map(|(k, _, _)| k.clone()).collect();
                if matches!(keys.get(id_at + 1), Some((_, _, true))) {
                    bare(&mut out, false);
                    continue;
                }
                out.references.push(Reference {
                    root: Root::Nodes(id.clone()),
                    root_text: slice(&text, root_start, base_end),
                    path,
                    head: shift(root_start, chain_end, offset),
                    optional,
                });
            }
        }
    }
    out.references.sort_by_key(|r| r.head.start);
    out.bare.sort_by_key(|b| b.at.start);
    Ok(out)
}

fn slice(text: &str, start: usize, end: usize) -> String {
    text.get(start..end).unwrap_or_default().to_string()
}

fn shift(start: usize, end: usize, offset: usize) -> Range<usize> {
    start.saturating_sub(offset)..end.saturating_sub(offset)
}

/// The default export's first parameter: a name bound to the whole payload,
/// or the keys a destructuring pattern takes from it.
fn page_props(semantic: &oxc_semantic::Semantic<'_>) -> (Vec<oxc_semantic::SymbolId>, Vec<String>) {
    use oxc_ast::ast::{BindingPattern, ExportDefaultDeclarationKind, PropertyKey, Statement};
    let program = semantic.nodes().program();
    let mut symbols = Vec::new();
    let mut destructured = Vec::new();
    for statement in &program.body {
        let Statement::ExportDefaultDeclaration(export) = statement else { continue };
        let ExportDefaultDeclarationKind::FunctionDeclaration(function) = &export.declaration else {
            continue;
        };
        let Some(first) = function.params.items.first() else { continue };
        match &first.pattern {
            BindingPattern::BindingIdentifier(ident) => {
                if let Some(symbol) = ident.symbol_id.get() {
                    symbols.push(symbol);
                }
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    match &property.key {
                        PropertyKey::StaticIdentifier(key) => destructured.push(key.name.to_string()),
                        _ => destructured.push("[computed]".to_string()),
                    }
                }
                if object.rest.is_some() {
                    destructured.push("...rest".to_string());
                }
            }
            _ => {}
        }
    }
    (symbols, destructured)
}

/// `source` with each `(range, replacement)` applied; ranges must not overlap.
pub fn apply_edits(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut out = source.to_string();
    for (range, replacement) in edits {
        if range.end <= out.len() && out.is_char_boundary(range.start) && out.is_char_boundary(range.end) {
            out.replace_range(range, &replacement);
        }
    }
    out
}

/// The `{{ … }}` expressions of a config string, as the engine's scanner
/// splits them: each expression's byte range inside `text`.
pub fn template_expressions(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find("{{") {
        let start = at + open + 2;
        let Some(close) = text[start..].find("}}") else { break };
        let end = start + close;
        if !text[start..end].trim().is_empty() {
            out.push(start..end);
        }
        at = end + 2;
    }
    out
}

/// Whether `text` is one whole `{{ expression }}` and nothing else.
pub fn whole_expression(text: &str) -> Option<&str> {
    let inner = text.trim().strip_prefix("{{")?.strip_suffix("}}")?;
    if inner.contains("{{") || inner.contains("}}") || inner.trim().is_empty() {
        return None;
    }
    Some(inner.trim())
}

/// The elements of an array literal expression (`[a, b]`), as source
/// text; `None` when the expression is anything else or spreads.
pub fn array_elements(expression: &str) -> Option<Vec<String>> {
    let text = format!("{EXPRESSION_PREFIX}{expression}{EXPRESSION_SUFFIX}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &text, SourceType::default().with_module(false)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return None;
    }
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = parsed.program.body.first()? else {
        return None;
    };
    let mut expr = &statement.expression;
    while let Expression::ParenthesizedExpression(inner) = expr {
        expr = &inner.expression;
    }
    let Expression::ArrayExpression(array) = expr else { return None };
    let mut out = Vec::new();
    for element in &array.elements {
        let span = match element {
            oxc_ast::ast::ArrayExpressionElement::SpreadElement(_) | oxc_ast::ast::ArrayExpressionElement::Elision(_) => {
                return None;
            }
            other => other.span(),
        };
        out.push(text[span.start as usize..span.end as usize].to_string());
    }
    Some(out)
}

/// Whether an expression is a plain member path (`input.params.id`,
/// `$trigger.body.x`) — one value, never a list built on the spot.
pub fn is_member_path(expression: &str) -> bool {
    let text = format!("{EXPRESSION_PREFIX}{expression}{EXPRESSION_SUFFIX}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &text, SourceType::default().with_module(false)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return false;
    }
    let Some(oxc_ast::ast::Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
        return false;
    };
    let mut expr = &statement.expression;
    loop {
        match expr {
            Expression::ParenthesizedExpression(inner) => expr = &inner.expression,
            Expression::StaticMemberExpression(member) => expr = &member.object,
            Expression::ComputedMemberExpression(member) => {
                if !matches!(member.expression, Expression::StringLiteral(_) | Expression::NumericLiteral(_)) {
                    return false;
                }
                expr = &member.object;
            }
            Expression::ChainExpression(_) => return false,
            Expression::Identifier(_) => return true,
            _ => return false,
        }
    }
}

/// Whether an expression is an object or array literal (its value is never
/// a string).
pub fn is_object_literal(expression: &str) -> bool {
    let text = format!("{EXPRESSION_PREFIX}{expression}{EXPRESSION_SUFFIX}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &text, SourceType::default().with_module(false)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return false;
    }
    let Some(oxc_ast::ast::Statement::ExpressionStatement(statement)) = parsed.program.body.first() else {
        return false;
    };
    let mut expr = &statement.expression;
    while let Expression::ParenthesizedExpression(inner) = expr {
        expr = &inner.expression;
    }
    matches!(expr, Expression::ObjectExpression(_) | Expression::ArrayExpression(_))
}

/// What a script body returns: `Some(true)` when every `return` gives an
/// object or array literal, `Some(false)` when some return gives anything
/// else, `None` when it does not parse.
pub fn returns_only_literals(source: &str) -> Option<bool> {
    let text = format!("{BODY_PREFIX}{source}{BODY_SUFFIX}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &text, SourceType::default().with_module(false)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return None;
    }
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let outer = semantic
        .nodes()
        .iter()
        .find(|n| matches!(n.kind(), AstKind::Function(f) if f.id.as_ref().is_some_and(|id| id.name == "__zf_migrate")))?
        .id();
    let mut all = true;
    for node in semantic.nodes().iter() {
        let AstKind::ReturnStatement(ret) = node.kind() else { continue };
        // Only the wrapper's own returns, not those of inner functions.
        let owner = semantic
            .nodes()
            .ancestors(node.id())
            .find(|a| matches!(a.kind(), AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)))
            .map(|a| a.id());
        if owner != Some(outer) {
            continue;
        }
        let literal = ret.argument.as_ref().is_some_and(|arg| {
            let mut e = arg;
            while let Expression::ParenthesizedExpression(inner) = e {
                e = &inner.expression;
            }
            matches!(e, Expression::ObjectExpression(_) | Expression::ArrayExpression(_))
        });
        all &= literal;
    }
    Some(all)
}

/// What a script body returns, when every `return` of its own is an object
/// literal with written keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Returned {
    /// Every key any return gives.
    pub any: std::collections::BTreeSet<String>,
    /// The keys every return gives.
    pub always: std::collections::BTreeSet<String>,
    /// Every return starts `{ ...input, … }`: the payload it was given,
    /// with these keys on top.
    pub spreads_input: bool,
}

/// The keys a script body returns: every key any return gives and the keys
/// every return gives. `None` when that cannot be said.
pub fn returned_keys(source: &str) -> Option<(std::collections::BTreeSet<String>, std::collections::BTreeSet<String>)> {
    let returned = returned(source)?;
    if returned.spreads_input {
        return None;
    }
    Some((returned.any, returned.always))
}

/// What a script body returns (see [`Returned`]); `None` when a return is
/// anything but such a literal, or some returns spread the payload and
/// others do not.
pub fn returned(source: &str) -> Option<Returned> {
    use oxc_ast::ast::{ObjectPropertyKind, PropertyKey};
    let text = format!("{BODY_PREFIX}{source}{BODY_SUFFIX}");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &text, SourceType::default().with_module(false)).parse();
    if parsed.panicked || !parsed.errors.is_empty() {
        return None;
    }
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let outer = semantic
        .nodes()
        .iter()
        .find(|n| matches!(n.kind(), AstKind::Function(f) if f.id.as_ref().is_some_and(|id| id.name == "__zf_migrate")))?
        .id();
    // `const base = input` (or `input || {}`, `input ?? {}`, or a
    // conditional between `input` and `{}`): spreading it spreads the
    // payload.
    let mut aliases: Vec<String> = Vec::new();
    for node in semantic.nodes().iter() {
        let AstKind::VariableDeclaration(declaration) = node.kind() else { continue };
        if declaration.kind != oxc_ast::ast::VariableDeclarationKind::Const {
            continue;
        }
        for declarator in &declaration.declarations {
            let (oxc_ast::ast::BindingPattern::BindingIdentifier(id), Some(init)) = (&declarator.id, &declarator.init) else {
                continue;
            };
            if input_like(init) {
                aliases.push(id.name.to_string());
            }
        }
    }
    let mut any = std::collections::BTreeSet::new();
    let mut always: Option<std::collections::BTreeSet<String>> = None;
    let mut spreads: Option<bool> = None;
    let mut returns = 0;
    for node in semantic.nodes().iter() {
        let AstKind::ReturnStatement(ret) = node.kind() else { continue };
        let owner = semantic
            .nodes()
            .ancestors(node.id())
            .find(|a| matches!(a.kind(), AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)))
            .map(|a| a.id());
        if owner != Some(outer) {
            continue;
        }
        returns += 1;
        let mut e = ret.argument.as_ref()?;
        while let Expression::ParenthesizedExpression(inner) = e {
            e = &inner.expression;
        }
        let Expression::ObjectExpression(object) = e else { return None };
        let mut keys = std::collections::BTreeSet::new();
        let mut spread = false;
        for (position, property) in object.properties.iter().enumerate() {
            let property = match property {
                ObjectPropertyKind::SpreadProperty(s)
                    if position == 0
                        && matches!(&s.argument, Expression::Identifier(id) if id.name == "input" || aliases.iter().any(|a| a == id.name.as_str())) =>
                {
                    spread = true;
                    continue;
                }
                ObjectPropertyKind::ObjectProperty(property) => property,
                _ => return None,
            };
            if property.computed {
                return None;
            }
            match &property.key {
                PropertyKey::StaticIdentifier(id) => keys.insert(id.name.to_string()),
                PropertyKey::StringLiteral(lit) => keys.insert(lit.value.to_string()),
                _ => return None,
            };
        }
        match spreads {
            None => spreads = Some(spread),
            Some(previous) if previous != spread => return None,
            _ => {}
        }
        any.extend(keys.iter().cloned());
        always = Some(match always {
            None => keys,
            Some(prev) => prev.intersection(&keys).cloned().collect(),
        });
    }
    if returns == 0 {
        return None;
    }
    Some(Returned { any, always: always.unwrap_or_default(), spreads_input: spreads.unwrap_or(false) })
}

/// Whether an expression is the script's payload, or the payload falling
/// back to an empty object.
fn input_like(expr: &Expression<'_>) -> bool {
    let is_input = |e: &Expression<'_>| matches!(e, Expression::Identifier(id) if id.name == "input");
    let is_empty_object = |e: &Expression<'_>| matches!(e, Expression::ObjectExpression(o) if o.properties.is_empty());
    match expr {
        Expression::ParenthesizedExpression(inner) => input_like(&inner.expression),
        Expression::Identifier(_) => is_input(expr),
        Expression::LogicalExpression(l) => is_input(&l.left) && is_empty_object(&l.right),
        Expression::ConditionalExpression(c) => {
            (input_like(&c.consequent) && is_empty_object(&c.alternate))
                || (is_empty_object(&c.consequent) && input_like(&c.alternate))
        }
        _ => false,
    }
}

/// The project files a page or component imports (`@/…` from the source
/// root, `./…` from its own folder), as source-relative paths without an
/// extension decided.
pub fn local_imports(source: &str, own_dir: &str) -> Vec<String> {
    use oxc_ast::ast::Statement;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::default().with_module(true).with_jsx(true).with_typescript(true)).parse();
    let mut out = Vec::new();
    for statement in &parsed.program.body {
        let Statement::ImportDeclaration(import) = statement else { continue };
        let spec = import.source.value.as_str();
        let path = if let Some(rest) = spec.strip_prefix("@/") {
            rest.to_string()
        } else if spec.starts_with("./") || spec.starts_with("../") {
            let mut parts: Vec<&str> = own_dir.split('/').filter(|p| !p.is_empty()).collect();
            for piece in spec.split('/') {
                match piece {
                    "." => {}
                    ".." => {
                        parts.pop();
                    }
                    other => parts.push(other),
                }
            }
            parts.join("/")
        } else {
            continue;
        };
        out.push(path);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heads(source: &str, wrap: Wrap) -> Vec<(Root, Vec<String>, String)> {
        scan(source, wrap)
            .expect("parses")
            .references
            .into_iter()
            .map(|r| (r.root, r.path, source[r.head].to_string()))
            .collect()
    }

    #[test]
    fn an_expression_reads_input_and_nodes_but_not_strings_or_shadows() {
        let found = heads(
            "input.rows.map(input => input.id).concat($nodes.q.rows, 'input.body', $input.body?.name)",
            Wrap::Expression,
        );
        assert_eq!(
            found,
            vec![
                (Root::Input, vec!["rows".into(), "map".into()], "input.rows.map".into()),
                (Root::Nodes("q".into()), vec!["rows".into()], "$nodes.q.rows".into()),
                (Root::Input, vec!["body".into(), "name".into()], "$input.body?.name".into()),
            ]
        );
    }

    #[test]
    fn a_bare_or_computed_input_is_reported_not_rewritten() {
        let scan = scan("JSON.stringify(input) + input[key] + input['body']", Wrap::Expression).expect("parses");
        assert!(scan.references.is_empty(), "{:?}", scan.references);
        assert_eq!(scan.bare.len(), 3);
        assert!(scan.bare[0].whole);
        assert!(!scan.bare[1].whole);
    }

    #[test]
    fn a_script_body_reads_its_input_parameter_and_ctx_nodes() {
        let found = heads(
            "const b = input.body; const p = ctx.nodes['n2'].rows; return { id: b.id, at: input.params.id, t: ctx.trigger };",
            Wrap::FunctionBody,
        );
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(found[0].2, "input.body");
        assert_eq!(found[1].0, Root::Nodes("n2".into()));
        assert_eq!(found[1].2, "ctx.nodes['n2'].rows");
        assert_eq!(found[2].2, "input.params.id");
    }

    #[test]
    fn a_page_reads_its_first_parameter_and_the_input_global() {
        let named = "export default function Page(input) { return <p>{input.rows.length}</p>; }";
        assert_eq!(heads(named, Wrap::Page)[0].2, "input.rows.length");
        let props = "export default function Page(props) { const r = input?.rows ?? []; return <p>{props.title}</p>; }";
        let found = heads(props, Wrap::Page);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].2, "input?.rows");
        assert!(scan(props, Wrap::Page).unwrap().references[0].optional);
        assert_eq!(found[1].2, "props.title");
        let destructured = scan("export default function Page({ rows, params }) { return <p>{rows.length}</p>; }", Wrap::Page).unwrap();
        assert_eq!(destructured.destructured, vec!["rows".to_string(), "params".to_string()]);
    }

    #[test]
    fn a_bare_nodes_reference_is_a_reference_with_no_key() {
        let found = heads("$nodes.name + 1", Wrap::Expression);
        assert_eq!(found, vec![(Root::Nodes("name".into()), vec![], "$nodes.name".into())]);
    }

    #[test]
    fn edits_replace_only_the_head() {
        let source = "{{ input.rows[0].id }}";
        let expr = template_expressions(source)[0].clone();
        let inner = &source[expr.clone()];
        let reference = scan(inner, Wrap::Expression).unwrap().references.remove(0);
        let head = (reference.head.start + expr.start)..(reference.head.end + expr.start);
        assert_eq!(&source[head.clone()], "input.rows[0].id");
        assert_eq!(apply_edits(source, vec![(head, "input.query.rows[0].id".into())]), "{{ input.query.rows[0].id }}");
    }

    #[test]
    fn literal_shapes_are_recognised() {
        assert_eq!(array_elements("[input.a, f(1, 2)]"), Some(vec!["input.a".to_string(), "f(1, 2)".to_string()]));
        assert_eq!(array_elements("input.ids"), None);
        assert!(is_member_path("input.params['id']"));
        assert!(!is_member_path("input.a ?? null"));
        assert!(is_object_literal("{ ok: true }"));
        assert_eq!(returns_only_literals("if (x) return { a: 1 }; return [1];"), Some(true));
        assert_eq!(returns_only_literals("const f = () => 'x'; return input.rows;"), Some(false));
    }
}
