//! `web.site.generate --reference zebflow`: the Zebflow reference as pages of
//! a docs site, generated from the running Zebflow — never files in the
//! repository.
//!
//! - `nodes/` — every official node, one page each under its family
//!   (`nodes/trigger/trigger.repo`), rendered from its definition: the same
//!   text `help("pipeline/nodes/<kind>")` gives an agent.
//! - `examples/` — the worked recipes of `help("pipeline/examples")`.
//!
//! The two are linked both ways at build time: a node page lists the recipes
//! whose code uses it ("Used in"), and a node named in inline code anywhere
//! on the site (`` `trigger.repo` ``) links to its page. An authored page at
//! the address of a generated one is added to the bottom of it.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::docs::{DocPage, FolderMeta};
use super::Config;
use crate::pipeline::{NodeDefinition, PipelineError};

/// The one reference there is.
pub const ZEBFLOW: &str = "zebflow";
pub const CODE_REFERENCE: &str = "FW_NODE_WEB_SITE_GENERATE_REFERENCE";

/// The folders the reference writes, and where they sit in the sidebar after
/// the authored pages unless a `_meta.yaml` says otherwise.
const NODES: &str = "nodes";
const EXAMPLES: &str = "examples";

/// Whether the build includes the reference; refused for any other name.
pub fn wanted(config: &Config) -> Result<bool, PipelineError> {
    match config.reference.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(false),
        Some(ZEBFLOW) => Ok(true),
        Some(other) => Err(PipelineError::new(
            CODE_REFERENCE,
            format!("--reference '{other}' is not a reference this Zebflow has; the one there is is '{ZEBFLOW}'"),
        )),
    }
}

/// A node's family: `trigger` for `trigger.repo`.
fn family(kind: &str) -> &str {
    kind.split('.').next().unwrap_or(kind)
}

/// A node page's Markdown path under the docs folder.
pub fn node_rel(kind: &str) -> String {
    format!("{NODES}/{}/{kind}.md", family(kind))
}

fn example_rel(slug: &str) -> String {
    format!("{EXAMPLES}/{slug}.md")
}

/// The node kinds each recipe's code uses: every word of a fenced block that
/// is a known kind.
fn kinds_in_code(markdown: &str, kinds: &HashSet<String>) -> Vec<String> {
    let mut found = Vec::new();
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            continue;
        }
        for word in line.split(|c: char| c.is_whitespace() || c == '|') {
            if kinds.contains(word) && !found.iter().any(|k| k == word) {
                found.push(word.to_string());
            }
        }
    }
    found
}

/// One sentence, plain, for a meta description and an index line.
fn first_sentence(text: &str) -> String {
    let plain: String = text.chars().filter(|c| *c != '`' && *c != '*').collect();
    let sentence = plain.split(". ").next().unwrap_or(&plain).trim().trim_end_matches('.');
    sentence.replace('\n', " ")
}

/// A front matter value on one line, never read as YAML syntax.
fn front_value(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "").replace('"', "'"))
}

/// A node's public page: its own words from the definition, laid out for a
/// reader rather than an agent — sections a page can be navigated by, a flag
/// table of what a reader needs, each example under its own heading.
fn node_page(def: &NodeDefinition, used_in: &[(&str, &str)]) -> String {
    let mut page = format!(
        "---\ntitle: {}\ndescription: {}\n---\n\n# {}\n\n```\n{}\n```\n\n{}\n\n",
        front_value(&def.kind),
        front_value(&first_sentence(&def.description)),
        def.kind,
        crate::pipeline::nodes::node_signature(def),
        def.description.trim(),
    );
    let providers = crate::pipeline::nodes::format_provider_profiles_markdown(def);
    if !providers.trim().is_empty() {
        page.push_str(&providers);
        page.push('\n');
    }
    // The flags every node has are listed once, on the Nodes page.
    let common: Vec<String> = crate::pipeline::model::engine_common_dsl_flags().into_iter().map(|flag| flag.flag).collect();
    let own: Vec<_> = def.dsl_flags.iter().filter(|flag| !common.contains(&flag.flag)).collect();
    if !own.is_empty() {
        page.push_str("## Flags\n\n");
        for flag in own {
            let required = if flag.required { " (required)" } else { "" };
            page.push_str(&format!("- `{}`{required}: {}\n", flag.flag, flag.description.trim().replace('\n', " ")));
        }
        page.push('\n');
    }
    if !def.examples.is_empty() {
        page.push_str("## Examples\n\n");
        for (index, example) in def.examples.iter().enumerate() {
            let title = if example.title.trim().is_empty() { format!("Example {}", index + 1) } else { example.title.trim().to_string() };
            page.push_str(&format!("### {title}\n\n"));
            if !example.dsl.is_empty() {
                page.push_str(&format!("```\n| {}\n```\n\n", example.dsl));
            }
            if !example.description.trim().is_empty() {
                page.push_str(&format!("{}\n\n", example.description.trim()));
            }
            if !example.input.is_null() {
                page.push_str(&format!("Input:\n\n```json\n{}\n```\n\n", serde_json::to_string_pretty(&example.input).unwrap_or_default()));
            }
            if !example.output.is_null() {
                page.push_str(&format!("It answers:\n\n```json\n{}\n```\n\n", serde_json::to_string_pretty(&example.output).unwrap_or_default()));
            }
        }
    }
    if !used_in.is_empty() {
        page.push_str("## Used in\n\n");
        for (slug, title) in used_in {
            page.push_str(&format!("- [{title}](/{})\n", example_rel(slug)));
        }
    }
    page
}

fn nodes_index(defs: &[NodeDefinition]) -> String {
    let mut by_family: BTreeMap<&str, Vec<&NodeDefinition>> = BTreeMap::new();
    for def in defs {
        by_family.entry(family(&def.kind)).or_default().push(def);
    }
    let mut page = format!(
        "---\ntitle: \"Nodes\"\ndescription: \"Every node a Zebflow pipeline is built from, {} in all, grouped by family.\"\n---\n\n# Nodes\n\n\
         A pipeline is a line of nodes: `| trigger.webhook --route /hello | web.response.send --body \"hi\"`. Every node of this Zebflow, \
         {} in all, grouped by family. Each page is generated from the node itself, so it describes exactly the version this site runs.\n\n\
         ## Flags every node takes\n\n",
        defs.len(),
        defs.len()
    );
    for flag in crate::pipeline::model::engine_common_dsl_flags() {
        page.push_str(&format!("- `{}`: {}\n", flag.flag, flag.description.trim().replace('\n', " ")));
    }
    for (family, kinds) in &by_family {
        page.push_str(&format!("\n## {family}\n\n| Node | What it does |\n| --- | --- |\n"));
        for def in kinds {
            page.push_str(&format!("| [`{}`](/{}) | {} |\n", def.kind, node_rel(&def.kind), first_sentence(&def.description).replace('|', "/")));
        }
    }
    page
}

fn family_index(family: &str, kinds: &[&NodeDefinition]) -> String {
    let mut page = format!("---\ntitle: {}\n---\n\n# {family}\n\n", front_value(family));
    for def in kinds {
        page.push_str(&format!("- [`{}`](/{}): {}\n", def.kind, node_rel(&def.kind), first_sentence(&def.description)));
    }
    page
}

fn examples_index(recipes: &[(&str, &str, String)]) -> String {
    let mut page = String::from(
        "---\ntitle: \"Examples\"\ndescription: \"Worked pipelines, each built from several nodes.\"\n---\n\n# Examples\n\n\
         Worked pipelines, each built from several nodes. Every node named links to its page.\n\n",
    );
    for (slug, title, _) in recipes {
        page.push_str(&format!("- [{title}](/{})\n", example_rel(slug)));
    }
    page
}

/// The reference's pages, `(path under the docs folder, Markdown)`, and the
/// kinds there are.
pub fn sources() -> (Vec<(String, String)>, HashSet<String>) {
    let defs = crate::platform::help::official_node_definitions();
    let recipes = crate::platform::help::example_recipes();
    let kinds: HashSet<String> = defs.iter().map(|def| def.kind.clone()).collect();

    let mut used_in: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
    for (slug, title, markdown) in &recipes {
        for kind in kinds_in_code(markdown, &kinds) {
            used_in.entry(kind).or_default().push((slug, title));
        }
    }

    let mut out = vec![(format!("{NODES}/index.md"), nodes_index(&defs))];
    let mut by_family: BTreeMap<&str, Vec<&NodeDefinition>> = BTreeMap::new();
    for def in &defs {
        by_family.entry(family(&def.kind)).or_default().push(def);
        out.push((node_rel(&def.kind), node_page(def, used_in.get(&def.kind).map(Vec::as_slice).unwrap_or(&[]))));
    }
    for (family, kinds) in &by_family {
        out.push((format!("{NODES}/{family}/index.md"), family_index(family, kinds)));
    }
    out.push((format!("{EXAMPLES}/index.md"), examples_index(&recipes)));
    for (slug, _, markdown) in &recipes {
        out.push((example_rel(slug), markdown.clone()));
    }
    (out, kinds)
}

/// The sidebar titles of the reference's folders, where the site gives none.
pub fn folder_meta(meta: &mut HashMap<String, FolderMeta>) {
    for (folder, title, order) in [(NODES, "Nodes", 9_000), (EXAMPLES, "Examples", 8_000)] {
        let entry = meta.entry(folder.to_string()).or_default();
        entry.title.get_or_insert_with(|| title.to_string());
        entry.order.get_or_insert(order);
        if folder == NODES {
            entry.collapsed.get_or_insert(true);
        }
    }
}

/// `` `trigger.repo` `` in running text becomes a link to its page, outside
/// code blocks, headings and links, and never on the node's own page.
pub fn link_kinds(markdown: &str, kinds: &HashSet<String>, own: Option<&str>) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_fence = false;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
        }
        if in_fence || trimmed.starts_with("```") || trimmed.starts_with('#') {
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let Some(len) = rest[open + 1..].find('`') else { break };
            let code = &rest[open + 1..open + 1 + len];
            let end = open + 2 + len;
            let linked_already = rest[..open].ends_with('[');
            out.push_str(&rest[..open]);
            if !linked_already && kinds.contains(code) && own != Some(code) {
                out.push_str(&format!("[`{code}`](/{})", node_rel(code)));
            } else {
                out.push_str(&rest[open..end]);
            }
            rest = &rest[end..];
        }
        out.push_str(rest);
    }
    out
}

/// Adds the reference to an authored site: every generated page, an authored
/// page at a generated page's address added to the bottom of it, and every
/// node named in inline code linked to its page.
pub fn add_to(pages: &mut Vec<DocPage>, folder_meta_map: &mut HashMap<String, FolderMeta>, deploy_base_path: &str) -> Result<(), PipelineError> {
    let (sources, kinds) = sources();
    let mut authored: HashMap<Vec<String>, DocPage> =
        std::mem::take(pages).into_iter().map(|page| (page.slug_segments.clone(), page)).collect();
    for (rel, markdown) in sources {
        let own = rel.strip_prefix(&format!("{NODES}/")).and_then(|r| r.rsplit('/').next()).and_then(|f| f.strip_suffix(".md"));
        let mut page = super::docs::doc_page(&rel, &markdown, None, deploy_base_path)?;
        if let Some(addition) = authored.remove(&page.slug_segments) {
            let joined = format!("{}\n\n{}", markdown.trim_end(), addition.markdown);
            page = super::docs::doc_page(&rel, &joined, addition.source_modified, deploy_base_path)?;
        }
        relink(&mut page, &kinds, own.filter(|k| kinds.contains(*k)));
        pages.push(page);
    }
    for (_, mut page) in authored {
        relink(&mut page, &kinds, None);
        pages.push(page);
    }
    folder_meta(folder_meta_map);
    Ok(())
}

fn relink(page: &mut DocPage, kinds: &HashSet<String>, own: Option<&str>) {
    let linked = link_kinds(&page.markdown, kinds, own);
    if linked != page.markdown {
        page.blocks = super::docs::page_blocks(&linked, &page.title);
        page.markdown = linked;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds() -> HashSet<String> {
        ["trigger.repo", "web.site.generate"].iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_kind_in_running_text_links_and_code_stays_code() {
        let text = "Use `trigger.repo` with `web.site.generate`.\n\n```\n| trigger.repo --path docs\n```\n\n## `trigger.repo`\n[`trigger.repo`](/x.md) and `other.kind`\n";
        let out = link_kinds(text, &kinds(), Some("web.site.generate"));
        assert!(out.starts_with("Use [`trigger.repo`](/nodes/trigger/trigger.repo.md) with `web.site.generate`."), "{out}");
        assert!(out.contains("```\n| trigger.repo --path docs\n```"), "a code block is left alone: {out}");
        assert!(out.contains("## `trigger.repo`\n"), "a heading is left alone: {out}");
        assert!(out.contains("[`trigger.repo`](/x.md) and `other.kind`"), "a link is not linked again: {out}");
    }

    #[test]
    fn a_recipe_uses_the_kinds_its_code_names() {
        let recipe = "Text naming trigger.repo.\n\n```zf\n| trigger.repo --path docs\n| web.site.generate --from docs\n| trigger.repo --path x\n```\n";
        assert_eq!(kinds_in_code(recipe, &kinds()), vec!["trigger.repo", "web.site.generate"]);
    }

    #[test]
    fn only_zebflow_is_a_reference() {
        let with = |r: &str| Config { reference: Some(r.to_string()), ..Config::default() };
        assert!(wanted(&Config::default()).unwrap() == false);
        assert!(wanted(&with("zebflow")).unwrap());
        assert!(wanted(&with("elsewhere")).is_err());
    }

    /// Every node has a page under its family, the recipes have theirs, and a
    /// node a recipe uses lists it.
    #[test]
    fn the_reference_covers_every_node_and_links_recipes() {
        let (sources, kinds) = sources();
        let paths: HashSet<&str> = sources.iter().map(|(rel, _)| rel.as_str()).collect();
        for kind in &kinds {
            assert!(paths.contains(node_rel(kind).as_str()), "{kind} has a page");
        }
        assert!(paths.contains("nodes/index.md") && paths.contains("examples/index.md"));
        let generate = &sources.iter().find(|(rel, _)| rel == "nodes/web/web.site.generate.md").expect("page").1;
        assert!(generate.contains("\n## Flags\n") && generate.contains("\n## Examples\n") && generate.contains("\n### "), "a node page has sections: {generate}");
        assert!(!generate.contains("Config key") && !generate.contains("**MCP:**"), "the agent layout stays in help(): {generate}");
        assert!(!generate.contains("`--preview-in`"), "the flags every node has are on the Nodes page, not on each: {generate}");
        let index = &sources.iter().find(|(rel, _)| rel == "nodes/index.md").expect("index").1;
        assert!(index.contains("## Flags every node takes") && index.contains("`--preview-in`"), "{index}");
        assert!(generate.contains("## Used in\n") && generate.contains("(/examples/static-entry-generation.md)"), "{generate}");
    }

    /// A whole site with the reference: it builds (so no generated link is
    /// dead), an authored page names a node and links to it, and an authored
    /// page at a node's address is added to that node's page.
    #[test]
    fn a_site_with_the_reference_builds_and_links_both_ways() {
        let tmp = tempfile::Builder::new().prefix("zebflow-docs-reference-").tempdir().expect("tempdir");
        let docs = tmp.path().join("docs");
        let source = tmp.path().join("src");
        std::fs::create_dir_all(&source).expect("source");
        for (rel, content) in [
            ("index.md", "# Start\n\nRebuild with `trigger.repo` on every save.\n"),
            ("nodes/trigger/trigger.repo.md", "## A note from the docs team\n\nWatch one folder, not the whole repository.\n"),
        ] {
            let path = docs.join("site").join(rel);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dir");
            std::fs::write(path, content).expect("write");
        }
        let config = Config { from: Some("site".into()), reference: Some(ZEBFLOW.into()), ..Config::default() };
        let create = |rel: &str, content: &str| {
            crate::pipeline::nodes::shared::project_store::create_repo_file(&source, rel, content.as_bytes(), "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_WRITE")
        };
        let site = super::super::docs::load_site(&config, "docs", "/docs/", &source, &docs, &create).expect("the site builds");
        let page = |route: &str| site.pages.iter().find(|p| p.route_path == route).unwrap_or_else(|| panic!("{route}"));

        let rendered = |route: &str| serde_json::to_string(&page(route).blocks).expect("blocks");
        assert!(rendered("/docs/").contains("[`trigger.repo`](/docs/nodes/trigger/trigger.repo/)"), "{}", rendered("/docs/"));
        let node = page("/docs/nodes/trigger/trigger.repo/");
        assert_eq!(node.title, "trigger.repo");
        assert!(node.markdown.contains("A note from the docs team"), "the authored addition is on the node's page");
        assert_eq!(site.pages.iter().filter(|p| p.route_path == "/docs/nodes/trigger/trigger.repo/").count(), 1, "one page, not two");
        let generate = page("/docs/nodes/web/web.site.generate/");
        assert!(generate.markdown.contains("## Used in"), "{}", generate.markdown);
        assert!(rendered("/docs/nodes/web/web.site.generate/").contains("(/docs/examples/static-entry-generation/)"), "used in links to the example");
        assert!(rendered("/docs/examples/static-entry-generation/").contains("[`web.site.generate`](/docs/nodes/web/web.site.generate/)"), "the example links to its node");
        assert!(site.sidebar.iter().any(|item| item.title == "Nodes"), "the sidebar has the reference");
    }
}
