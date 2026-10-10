//! Every internal link and anchor of a docs site, checked against what the
//! build is about to write.
//!
//! The build already knows every route and every heading id, so a link to a
//! page that does not exist, or to a heading that was renamed, is knowable
//! before the site is written. It is reported as a failure of the build rather
//! than left for a reader to find: a site that is written with dead links is a
//! site that lies, and nothing downstream will catch it.
//!
//! External links are never fetched. What is outside the site is outside the
//! build's knowledge, and a build whose result depends on the network is not a
//! build.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::docs::DocPage;

/// One link the build could not resolve.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BrokenLink {
    /// The Markdown file holding it, relative to the docs folder.
    pub source_rel_path: String,
    /// The link target as written.
    pub target: String,
    /// What is wrong, in one line.
    pub reason: String,
}

/// The internal links of every page, resolved against the routes and headings
/// the build produced. Answers every broken one, sorted, rather than the first.
pub fn check(pages: &[DocPage]) -> Vec<BrokenLink> {
    let routes: HashSet<&str> = pages.iter().map(|page| page.route_path.as_str()).collect();
    let anchors: HashMap<&str, HashSet<&str>> = pages
        .iter()
        .map(|page| {
            (
                page.route_path.as_str(),
                page.headings.iter().map(|heading| heading.id.as_str()).collect(),
            )
        })
        .collect();

    let mut broken = BTreeSet::new();
    for page in pages {
        for target in link_targets(&page.markdown) {
            let trimmed = target.trim();
            if trimmed.is_empty() || is_external(trimmed) {
                continue;
            }
            let (path, anchor) = match trimmed.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor)),
                None => (trimmed, None),
            };
            // An image or a file beside the page is not a route, so it is not
            // this check's business: it is shipped by the asset path and
            // reported there if it is missing. Checking it here reported
            // every `![](diagram.png)` as a link that leaves the folder.
            if !path.is_empty() && !path.ends_with(".md") {
                continue;
            }

            // `#section` alone means a heading on this page.
            let route = if path.is_empty() {
                page.route_path.clone()
            } else {
                match resolve_route(page, path) {
                    Some(route) => route,
                    None => {
                        broken.insert(BrokenLink {
                            source_rel_path: page.source_rel_path.clone(),
                            target: trimmed.to_string(),
                            reason: "leaves the docs folder".to_string(),
                        });
                        continue;
                    }
                }
            };
            if !routes.contains(route.as_str()) {
                broken.insert(BrokenLink {
                    source_rel_path: page.source_rel_path.clone(),
                    target: trimmed.to_string(),
                    reason: format!("no page answers at {route}"),
                });
                continue;
            }
            if let Some(anchor) = anchor.map(str::trim).filter(|anchor| !anchor.is_empty())
                && !anchors.get(route.as_str()).is_some_and(|ids| ids.contains(anchor))
            {
                broken.insert(BrokenLink {
                    source_rel_path: page.source_rel_path.clone(),
                    target: trimmed.to_string(),
                    reason: format!("{route} has no heading '{anchor}'"),
                });
            }
        }
    }
    broken.into_iter().collect()
}

/// The one-line report a refused build carries.
pub fn report(broken: &[BrokenLink]) -> String {
    let mut out = format!(
        "{} broken {} in the docs folder:",
        broken.len(),
        if broken.len() == 1 { "link" } else { "links" }
    );
    for link in broken {
        out.push_str(&format!("\n  {} -> {} ({})", link.source_rel_path, link.target, link.reason));
    }
    out
}

fn is_external(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("//")
        || lower.starts_with("data:")
}

/// The route a relative Markdown link points at: `../guides/writing.md` from
/// `origins/index.md` is `/guides/writing/`. `None` when it climbs out of the
/// docs folder. Only a `.md` target reaches here; anything else is an asset,
/// not a route, and the caller has already left it alone.
fn resolve_route(page: &DocPage, path: &str) -> Option<String> {
    let mut segments: Vec<&str> = page.source_rel_path.split('/').collect();
    segments.pop(); // the page's own file name
    if path.starts_with('/') {
        segments.clear();
    }
    for part in path.trim_start_matches('/').split('/') {
        match part {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    let joined = segments.join("/");
    let mut slug: Vec<&str> = joined.trim_end_matches(".md").split('/').filter(|s| !s.is_empty()).collect();
    if slug.last() == Some(&"index") {
        slug.pop();
    }
    Some(if slug.is_empty() { "/".to_string() } else { format!("/{}/", slug.join("/")) })
}

/// Every `](target)` in the Markdown — links and images alike. Reference-style
/// links are not read: the generator's Markdown does not define them, and a
/// checker that half-understands a syntax reports noise.
fn link_targets(markdown: &str) -> Vec<String> {
    let bytes = markdown.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if in_fence {
            i += line.len() + 1;
            continue;
        }
        let start = i;
        let end = (i + line.len()).min(bytes.len());
        let mut cursor = start;
        while let Some(found) = markdown[cursor..end].find("](") {
            let open = cursor + found + 2;
            let Some(close_rel) = markdown[open..end].find(')') else {
                break;
            };
            let target = &markdown[open..open + close_rel];
            // `[text](url "title")` — the title is not part of the target.
            let target = target.split_whitespace().next().unwrap_or("");
            if !target.is_empty() {
                out.push(target.to_string());
            }
            cursor = open + close_rel + 1;
        }
        i += line.len() + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::nodes::basic::web::site::docs::DocHeading;

    fn page(rel: &str, markdown: &str, headings: &[&str]) -> DocPage {
        let slug: Vec<String> = {
            let mut parts: Vec<String> =
                rel.trim_end_matches(".md").split('/').filter(|s| !s.is_empty()).map(ToString::to_string).collect();
            if parts.last().map(|p| p == "index").unwrap_or(false) {
                parts.pop();
            }
            parts
        };
        let route = if slug.is_empty() { "/".to_string() } else { format!("/{}/", slug.join("/")) };
        DocPage {
            slug_segments: slug,
            title: rel.to_string(),
            description: String::new(),
            keywords: Vec::new(),
            canonical: None,
            noindex: false,
            blocks: super::super::docs_blocks::parse(markdown),
            markdown: markdown.to_string(),
            headings: headings
                .iter()
                .map(|id| DocHeading { level: 2, id: (*id).to_string(), text: (*id).to_string() })
                .collect(),
            source_rel_path: rel.to_string(),
            output_rel_path: format!("{}index.html", route.trim_start_matches('/')),
            route_path: route,
            order: None,
            source_modified: None,
        }
    }

    #[test]
    fn a_relative_link_between_pages_resolves() {
        let pages = vec![
            page("index.md", "See [guides](guides/writing.md) and [up](./index.md).", &[]),
            page("guides/writing.md", "Back to [home](../index.md).", &["front-matter"]),
        ];
        assert_eq!(check(&pages), Vec::new());
    }

    #[test]
    fn a_missing_page_and_a_missing_heading_are_both_reported() {
        let pages = vec![
            page("index.md", "[gone](guides/gone.md) and [heading](guides/writing.md#nowhere)", &[]),
            page("guides/writing.md", "nothing here", &["front-matter"]),
        ];
        let broken = check(&pages);
        assert_eq!(broken.len(), 2, "{broken:?}");
        assert!(broken.iter().any(|link| link.reason.contains("no page answers at /guides/gone/")));
        assert!(broken.iter().any(|link| link.reason.contains("has no heading 'nowhere'")));
    }

    #[test]
    fn an_anchor_on_this_page_is_checked_against_this_page() {
        let pages = vec![page("index.md", "[here](#the-proof) and [there](#missing)", &["the-proof"])];
        let broken = check(&pages);
        assert_eq!(broken.len(), 1);
        assert!(broken[0].reason.contains("has no heading 'missing'"), "{:?}", broken[0]);
    }

    #[test]
    fn external_links_and_code_blocks_are_left_alone() {
        let pages = vec![page(
            "index.md",
            "[out](https://example.com/gone) [mail](mailto:nobody@example.com)\n\n```md\n[in code](nowhere.md)\n```\n",
            &[],
        )];
        assert_eq!(check(&pages), Vec::new());
    }

    /// An image is shipped by the asset path, not answered by a route. This
    /// used to report every picture in the docs as a link leaving the folder.
    #[test]
    fn a_file_that_is_not_a_page_is_not_a_link() {
        let pages = vec![page(
            "index.md",
            "![A diagram](diagram.png)\n\n<PotoPlayer src=\"stories/intro.poto\" />\n\n[a download](../files/report.pdf)",
            &[],
        )];
        assert_eq!(check(&pages), Vec::new());
    }

    /// A link that climbs above the docs folder is refused rather than
    /// silently resolved against the filesystem.
    #[test]
    fn a_link_out_of_the_folder_is_reported() {
        let pages = vec![page("index.md", "[escape](../../secrets.md)", &[])];
        let broken = check(&pages);
        assert_eq!(broken.len(), 1);
        assert_eq!(broken[0].reason, "leaves the docs folder");
    }
}
