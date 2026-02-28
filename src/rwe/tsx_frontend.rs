//! TSX frontend lowering for RWE.
//!
//! This module parses TSX sources and lowers them into template/control parts
//! compatible with the current RWE compile pipeline. It also resolves a
//! compile-local component import graph rooted at `ReactiveWebOptions.templates`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use swc_common::{FileName, SourceMap, Span, Spanned, sync::Lrc};
use swc_ecma_ast::{
    BlockStmt, Decl, DefaultDecl, EsVersion, Expr, ImportDecl, ImportSpecifier, KeyValueProp, Lit,
    Module, ModuleDecl, ModuleItem, ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt,
};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer};

/// Lowered parts extracted from one TSX source.
#[derive(Debug, Clone)]
pub struct LoweredTsxTemplate {
    /// Lowered HTML template body.
    pub html_template: String,
    /// Optional control script source (`return {...};`).
    pub control_script_source: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct PageContract {
    head_title: Option<String>,
    head_description: Option<String>,
    head_meta: Vec<BTreeMap<String, String>>,
    head_links: Vec<BTreeMap<String, String>>,
    head_scripts: Vec<BTreeMap<String, String>>,
    html_lang: Option<String>,
    html_class_name: Option<String>,
    body_class_name: Option<String>,
    navigation: Option<String>,
}

/// Compile-scoped registry resolved from explicit TSX imports.
#[derive(Debug, Clone, Default)]
pub struct ResolvedComponentRegistry {
    /// Imported components keyed by the local TSX tag name used by the importer.
    pub registry: BTreeMap<String, String>,
}

#[derive(Debug)]
struct ParsedTsx {
    app_expr: Option<String>,
    page_contract: Option<PageContract>,
    jsx_expr: String,
}

#[derive(Debug, Clone)]
struct ParsedImport {
    source: String,
    is_type_only: bool,
    local_names: Vec<String>,
}

/// Returns `true` when source likely contains TSX authoring shape.
pub fn looks_like_tsx_source(source: &str, source_path: Option<&Path>) -> bool {
    if let Some(path) = source_path
        && path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tsx"))
    {
        return true;
    }
    source.contains("export default function") && source.contains("return (")
}

/// Resolves compile-time component imports for one TSX entry file.
///
/// Supported import shapes:
///
/// - `import Button from "@/components/ui/button";`
/// - `import Button from "@/components/ui/button";`
///
/// Current boundary:
///
/// - only local `./`, `../`, and `@/` imports
/// - only `.tsx` component modules
/// - every compile call must stay within `template_root`
pub fn resolve_component_imports(
    source: &str,
    source_path: &Path,
    template_root: &Path,
) -> Result<ResolvedComponentRegistry, String> {
    let entry_path = fs::canonicalize(source_path).map_err(|err| {
        format!(
            "failed resolving canonical entry path '{}': {err}",
            source_path.display()
        )
    })?;
    let root_path = fs::canonicalize(template_root).map_err(|err| {
        format!(
            "failed resolving canonical template root '{}': {err}",
            template_root.display()
        )
    })?;
    ensure_within_root(&entry_path, &root_path)?;

    let mut registry = BTreeMap::new();
    let mut seen_paths = BTreeSet::new();
    let mut names_to_paths = BTreeMap::<String, PathBuf>::new();

    walk_component_imports(
        source,
        &entry_path,
        &root_path,
        &mut registry,
        &mut seen_paths,
        &mut names_to_paths,
    )?;

    Ok(ResolvedComponentRegistry { registry })
}

/// Lowers TSX source into HTML/control parts.
pub fn lower_tsx_source_to_parts(source: &str) -> Result<LoweredTsxTemplate, String> {
    let parsed = parse_real_tsx(source, "rwe_template.tsx")?;
    let jsx = strip_wrapping_parens(parsed.jsx_expr.trim());
    let html_template = jsx_to_rwe_html(&jsx);
    let html_template = if let Some(page_contract) = &parsed.page_contract {
        let page_inner = extract_page_root_children(&html_template)?;
        wrap_page_document(&page_inner, page_contract)
    } else {
        html_template
    };

    let control_script_source = parsed.app_expr.map(|expr| {
        let clean = expr.trim().trim_end_matches(';');
        format!("return {clean};")
    });

    Ok(LoweredTsxTemplate {
        html_template,
        control_script_source,
    })
}

fn walk_component_imports(
    source: &str,
    source_path: &Path,
    template_root: &Path,
    registry: &mut BTreeMap<String, String>,
    seen_paths: &mut BTreeSet<PathBuf>,
    names_to_paths: &mut BTreeMap<String, PathBuf>,
) -> Result<(), String> {
    if !seen_paths.insert(source_path.to_path_buf()) {
        return Ok(());
    }

    let imports = collect_imports(source, source_path)?;
    for import in imports {
        if import.is_type_only || import.local_names.is_empty() {
            continue;
        }

        let resolved_path = resolve_import_path(&import.source, source_path, template_root)?;
        let extension = resolved_path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        if extension != "tsx" {
            return Err(format!(
                "unsupported template import '{}' from '{}': only .tsx components are supported in the current RWE compile stage",
                import.source,
                source_path.display()
            ));
        }

        let component_source = fs::read_to_string(&resolved_path).map_err(|err| {
            format!(
                "failed reading imported component '{}' from '{}': {err}",
                import.source,
                source_path.display()
            )
        })?;

        for local_name in &import.local_names {
            if let Some(existing_path) = names_to_paths.get(local_name)
                && existing_path != &resolved_path
            {
                return Err(format!(
                    "component import name '{}' collides between '{}' and '{}'; use one canonical component name per compile graph",
                    local_name,
                    existing_path.display(),
                    resolved_path.display()
                ));
            }
            names_to_paths.insert(local_name.clone(), resolved_path.clone());
            registry.insert(local_name.clone(), component_source.clone());
        }

        walk_component_imports(
            &component_source,
            &resolved_path,
            template_root,
            registry,
            seen_paths,
            names_to_paths,
        )?;
    }

    Ok(())
}

fn ensure_within_root(path: &Path, root: &Path) -> Result<(), String> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "path '{}' escapes template root '{}'",
            path.display(),
            root.display()
        ))
    }
}

fn resolve_import_path(
    import_source: &str,
    source_path: &Path,
    template_root: &Path,
) -> Result<PathBuf, String> {
    let base = if let Some(stripped) = import_source.strip_prefix("@/") {
        template_root.join(stripped)
    } else if import_source.starts_with("./") || import_source.starts_with("../") {
        source_path
            .parent()
            .ok_or_else(|| {
                format!(
                    "cannot resolve relative import '{}' from '{}'",
                    import_source,
                    source_path.display()
                )
            })?
            .join(import_source)
    } else {
        return Err(format!(
            "unsupported import source '{}' in '{}': only relative paths and '@/...' are supported",
            import_source,
            source_path.display()
        ));
    };

    let candidates = import_candidates(base);
    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let canonical = fs::canonicalize(&candidate)
            .map_err(|err| format!("failed resolving import '{}': {err}", candidate.display()))?;
        ensure_within_root(&canonical, template_root)?;
        return Ok(canonical);
    }

    Err(format!(
        "import '{}' from '{}' did not resolve under template root '{}'",
        import_source,
        source_path.display(),
        template_root.display()
    ))
}

fn import_candidates(base: PathBuf) -> Vec<PathBuf> {
    let has_extension = base.extension().is_some();
    let mut candidates = Vec::new();
    if has_extension {
        candidates.push(base);
    } else {
        candidates.push(base.clone().with_extension("tsx"));
        candidates.push(base.clone().with_extension("ts"));
        candidates.push(base.join("index.tsx"));
        candidates.push(base.join("index.ts"));
    }
    candidates
}

fn collect_imports(source: &str, source_path: &Path) -> Result<Vec<ParsedImport>, String> {
    let module = parse_module(source, &source_path.display().to_string())?;
    let mut imports = Vec::new();
    for item in &module.body {
        let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item else {
            continue;
        };
        imports.push(parse_import_decl(import_decl)?);
    }
    Ok(imports)
}

fn parse_import_decl(import: &ImportDecl) -> Result<ParsedImport, String> {
    let source = import
        .src
        .value
        .as_str()
        .ok_or_else(|| "non-utf8 import sources are not supported".to_string())?
        .to_string();

    let mut local_names = Vec::new();
    for specifier in &import.specifiers {
        match specifier {
            ImportSpecifier::Default(default) => {
                local_names.push(default.local.sym.to_string());
            }
            ImportSpecifier::Named(named) => {
                local_names.push(named.local.sym.to_string());
            }
            ImportSpecifier::Namespace(namespace) => {
                return Err(format!(
                    "namespace import '{}' is not supported for RWE template imports",
                    namespace.local.sym
                ));
            }
        }
    }

    Ok(ParsedImport {
        source,
        is_type_only: import.type_only,
        local_names,
    })
}

fn parse_real_tsx(source: &str, file_name: &str) -> Result<ParsedTsx, String> {
    let module = parse_module(source, file_name)?;

    let mut app_span: Option<Span> = None;
    let mut page_contract: Option<PageContract> = None;
    let mut jsx_span: Option<Span> = None;

    for item in &module.body {
        if app_span.is_none()
            && let Some(span) = find_exported_app_span(item)
        {
            app_span = Some(span);
        }
        if page_contract.is_none()
            && let Some(contract) = find_exported_page_contract(item)?
        {
            page_contract = Some(contract);
        }
        if jsx_span.is_none()
            && let Some(span) = find_default_page_return_span(item)
        {
            jsx_span = Some(span);
        }
    }

    let jsx_span = jsx_span.ok_or_else(|| {
        "missing `export default function Page(...) { return (...) }`".to_string()
    })?;

    let app_expr = app_span
        .map(|span| slice_by_span(source, span))
        .transpose()?;
    let jsx_expr = slice_by_span(source, jsx_span)?;
    if page_contract.is_some() && !contains_page_root(&jsx_expr) {
        return Err(
            "page templates must return a single `<Page>...</Page>` root when `export const page = {...}` is present"
                .to_string(),
        );
    }
    if page_contract.is_none() && contains_page_root(&jsx_expr) {
        return Err(
            "page templates using `<Page>...</Page>` must define `export const page = { ... }`"
                .to_string(),
        );
    }

    Ok(ParsedTsx {
        app_expr,
        page_contract,
        jsx_expr,
    })
}

fn parse_module(source: &str, file_name: &str) -> Result<Module, String> {
    let cm: Lrc<SourceMap> = Default::default();
    let file = cm.new_source_file(
        FileName::Custom(file_name.to_string()).into(),
        source.to_string(),
    );

    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: true,
            ..Default::default()
        }),
        EsVersion::Es2022,
        StringInput::from(&*file),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_module()
        .map_err(|e| format!("tsx parse failed: {e:?}"))?;
    let errs = parser.take_errors();
    if !errs.is_empty() {
        return Err(format!("tsx parse emitted {} errors", errs.len()));
    }

    Ok(module)
}

fn find_exported_app_span(item: &ModuleItem) -> Option<Span> {
    let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) = item else {
        return None;
    };
    let Decl::Var(var_decl) = &export_decl.decl else {
        return None;
    };
    for decl in &var_decl.decls {
        let Pat::Ident(ident) = &decl.name else {
            continue;
        };
        if ident.id.sym.as_ref() == "app"
            && let Some(init) = &decl.init
        {
            return Some(init.span());
        }
    }
    None
}

fn find_exported_page_contract(item: &ModuleItem) -> Result<Option<PageContract>, String> {
    let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) = item else {
        return Ok(None);
    };
    let Decl::Var(var_decl) = &export_decl.decl else {
        return Ok(None);
    };
    for decl in &var_decl.decls {
        let Pat::Ident(ident) = &decl.name else {
            continue;
        };
        if ident.id.sym.as_ref() != "page" {
            continue;
        }
        let Some(init) = &decl.init else {
            return Err("`export const page` must be initialized".to_string());
        };
        return parse_page_contract_expr(init).map(Some);
    }
    Ok(None)
}

fn find_default_page_return_span(item: &ModuleItem) -> Option<Span> {
    let ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export_default)) = item else {
        return None;
    };
    let DefaultDecl::Fn(func) = &export_default.decl else {
        return None;
    };
    let body = func.function.body.as_ref()?;
    find_return_expr_span(body)
}

fn find_return_expr_span(body: &BlockStmt) -> Option<Span> {
    for stmt in &body.stmts {
        match stmt {
            Stmt::Return(ret) => {
                if let Some(expr) = &ret.arg {
                    return Some(expr.span());
                }
            }
            Stmt::Block(block) => {
                for nested in &block.stmts {
                    if let Stmt::Return(ret) = nested
                        && let Some(expr) = &ret.arg
                    {
                        return Some(expr.span());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn slice_by_span(source: &str, span: Span) -> Result<String, String> {
    let lo = span.lo.0 as usize;
    let hi = span.hi.0 as usize;
    if lo == 0 || hi == 0 {
        return Err("invalid zero-based span".to_string());
    }
    let start = lo - 1;
    let end = hi - 1;
    if start > source.len() || end > source.len() || start > end {
        return Err("span out of source range".to_string());
    }
    Ok(source[start..end].to_string())
}

fn parse_page_contract_expr(expr: &Expr) -> Result<PageContract, String> {
    let Expr::Object(object) = expr else {
        return Err("`export const page` must be an object literal".to_string());
    };

    let mut contract = PageContract::default();
    for prop in &object.props {
        let PropOrSpread::Prop(prop) = prop else {
            return Err("spread syntax is not supported in `export const page`".to_string());
        };
        let Prop::KeyValue(kv) = &**prop else {
            return Err(
                "only key-value properties are supported in `export const page`".to_string(),
            );
        };
        let key = prop_name_to_string(&kv.key)?;
        match key.as_str() {
            "head" => parse_page_head(kv, &mut contract)?,
            "html" => parse_page_html(kv, &mut contract)?,
            "body" => parse_page_body(kv, &mut contract)?,
            "navigation" => {
                contract.navigation = Some(expect_string_expr(&kv.value, "page.navigation")?);
            }
            _ => {}
        }
    }
    Ok(contract)
}

fn parse_page_head(kv: &KeyValueProp, contract: &mut PageContract) -> Result<(), String> {
    let object = expect_object_expr(&kv.value, "page.head")?;
    for prop in &object.props {
        let PropOrSpread::Prop(prop) = prop else {
            return Err("spread syntax is not supported in `page.head`".to_string());
        };
        let Prop::KeyValue(kv) = &**prop else {
            return Err("only key-value properties are supported in `page.head`".to_string());
        };
        match prop_name_to_string(&kv.key)?.as_str() {
            "title" => {
                contract.head_title = Some(expect_string_expr(&kv.value, "page.head.title")?)
            }
            "description" => {
                contract.head_description =
                    Some(expect_string_expr(&kv.value, "page.head.description")?)
            }
            "meta" => contract.head_meta = parse_string_object_array(&kv.value, "page.head.meta")?,
            "links" => {
                contract.head_links = parse_string_object_array(&kv.value, "page.head.links")?
            }
            "scripts" => {
                contract.head_scripts = parse_string_object_array(&kv.value, "page.head.scripts")?
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_page_html(kv: &KeyValueProp, contract: &mut PageContract) -> Result<(), String> {
    let object = expect_object_expr(&kv.value, "page.html")?;
    for prop in &object.props {
        let PropOrSpread::Prop(prop) = prop else {
            return Err("spread syntax is not supported in `page.html`".to_string());
        };
        let Prop::KeyValue(kv) = &**prop else {
            return Err("only key-value properties are supported in `page.html`".to_string());
        };
        match prop_name_to_string(&kv.key)?.as_str() {
            "lang" => contract.html_lang = Some(expect_string_expr(&kv.value, "page.html.lang")?),
            "className" => {
                contract.html_class_name =
                    Some(expect_string_expr(&kv.value, "page.html.className")?)
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_page_body(kv: &KeyValueProp, contract: &mut PageContract) -> Result<(), String> {
    let object = expect_object_expr(&kv.value, "page.body")?;
    for prop in &object.props {
        let PropOrSpread::Prop(prop) = prop else {
            return Err("spread syntax is not supported in `page.body`".to_string());
        };
        let Prop::KeyValue(kv) = &**prop else {
            return Err("only key-value properties are supported in `page.body`".to_string());
        };
        if prop_name_to_string(&kv.key)?.as_str() == "className" {
            contract.body_class_name = Some(expect_string_expr(&kv.value, "page.body.className")?);
        }
    }
    Ok(())
}

fn expect_object_expr<'a>(expr: &'a Expr, field_name: &str) -> Result<&'a ObjectLit, String> {
    let Expr::Object(object) = expr else {
        return Err(format!("`{field_name}` must be an object literal"));
    };
    Ok(object)
}

fn expect_string_expr(expr: &Expr, field_name: &str) -> Result<String, String> {
    match expr {
        Expr::Lit(Lit::Str(s)) => Ok(s.value.as_str().unwrap_or_default().to_string()),
        _ => Err(format!("`{field_name}` must be a string literal")),
    }
}

fn prop_name_to_string(prop_name: &PropName) -> Result<String, String> {
    match prop_name {
        PropName::Ident(ident) => Ok(ident.sym.to_string()),
        PropName::Str(s) => Ok(s.value.as_str().unwrap_or_default().to_string()),
        _ => Err("computed keys are not supported in page metadata".to_string()),
    }
}

fn parse_string_object_array(
    expr: &Expr,
    field_name: &str,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let Expr::Array(array) = expr else {
        return Err(format!("`{field_name}` must be an array literal"));
    };
    let mut out = Vec::new();
    for element in &array.elems {
        let Some(element) = element else {
            continue;
        };
        let Expr::Object(object) = &*element.expr else {
            return Err(format!(
                "every `{field_name}` item must be an object literal"
            ));
        };
        let mut attrs = BTreeMap::new();
        for prop in &object.props {
            let PropOrSpread::Prop(prop) = prop else {
                return Err(format!(
                    "spread syntax is not supported in `{field_name}` items"
                ));
            };
            let Prop::KeyValue(kv) = &**prop else {
                return Err(format!(
                    "only key-value properties are supported in `{field_name}` items"
                ));
            };
            attrs.insert(
                prop_name_to_string(&kv.key)?,
                expect_string_expr(&kv.value, field_name)?,
            );
        }
        out.push(attrs);
    }
    Ok(out)
}

fn strip_wrapping_parens(input: &str) -> String {
    let mut s = input.trim();
    while s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        s = s[1..s.len() - 1].trim();
    }
    s.to_string()
}

fn contains_page_root(jsx_expr: &str) -> bool {
    jsx_expr.contains("<Page") && jsx_expr.contains("</Page>")
}

fn extract_page_root_children(html_template: &str) -> Result<String, String> {
    let trimmed = html_template.trim();
    if !trimmed.starts_with("<Page") {
        return Err("page templates must start with `<Page>`".to_string());
    }
    let Some(open_end) = trimmed.find('>') else {
        return Err("malformed `<Page>` root".to_string());
    };
    let close_start = trimmed
        .rfind("</Page>")
        .ok_or_else(|| "page templates must end with `</Page>`".to_string())?;
    let trailing = trimmed[close_start + "</Page>".len()..].trim();
    if !trailing.is_empty() {
        return Err("page templates may not contain trailing markup after `</Page>`".to_string());
    }
    Ok(trimmed[open_end + 1..close_start].trim().to_string())
}

fn wrap_page_document(body_inner: &str, page: &PageContract) -> String {
    let mut head = String::new();
    if let Some(title) = &page.head_title {
        head.push_str("<title>");
        head.push_str(title);
        head.push_str("</title>");
    }
    if let Some(description) = &page.head_description {
        head.push_str("<meta name=\"description\" content=\"");
        head.push_str(description);
        head.push_str("\" />");
    }
    if let Some(navigation) = &page.navigation {
        head.push_str("<meta name=\"zebflow-navigation\" content=\"");
        head.push_str(navigation);
        head.push_str("\" />");
    }
    for attrs in &page.head_meta {
        head.push_str("<meta");
        head.push_str(&render_attr_map(attrs));
        head.push_str(" />");
    }
    for attrs in &page.head_links {
        head.push_str("<link");
        head.push_str(&render_attr_map(attrs));
        head.push_str(" />");
    }
    for attrs in &page.head_scripts {
        head.push_str("<script");
        head.push_str(&render_attr_map(attrs));
        head.push_str("></script>");
    }

    let html_lang = attr_fragment("lang", page.html_lang.as_deref());
    let html_class = attr_fragment("class", page.html_class_name.as_deref());
    let body_class = attr_fragment("class", page.body_class_name.as_deref());

    format!(
        "<html{html_lang}{html_class}><head>{head}</head><body{body_class}>{body_inner}</body></html>"
    )
}

fn attr_fragment(name: &str, value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    format!(" {name}=\"{value}\"")
}

fn render_attr_map(attrs: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (key, value) in attrs {
        if value.trim().is_empty() {
            continue;
        }
        out.push(' ');
        out.push_str(key);
        out.push_str("=\"");
        out.push_str(value);
        out.push('"');
    }
    out
}

fn jsx_to_rwe_html(jsx: &str) -> String {
    let converted = jsx
        .replace("className=\"", "class=\"")
        .replace("onClick=\"", "@click=\"")
        .replace("onInput=\"", "@input=\"")
        .replace("onChange=\"", "@change=\"")
        .replace("onSubmit=\"", "@submit=\"")
        .replace("jFor=\"", "j-for=\"")
        .replace("jKey=\"", "j-key=\"")
        .replace("jShow=\"", "j-show=\"")
        .replace("jHide=\"", "j-hide=\"")
        .replace("jText=\"", "j-text=\"")
        .replace("jModel=\"", "j-model=\"")
        .replace("jAttrClass=\"", "j-attr:class=\"");
    restore_placeholders(&converted)
}

fn restore_placeholders(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] != b'}' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'}' {
                let expr = input[i + 1..j].trim();
                if is_path_like_expr(expr) {
                    out.push_str("{{");
                    out.push_str(expr);
                    out.push_str("}}");
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn is_path_like_expr(expr: &str) -> bool {
    if expr.is_empty() {
        return false;
    }
    expr.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$' | '[' | ']' | '"' | '\''))
}
