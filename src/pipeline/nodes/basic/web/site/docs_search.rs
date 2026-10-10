//! The search index a docs site carries: ordered chunks on disk, so a query
//! fetches the part of the index its own terms live in and nothing else.
//!
//! One flat file would be simpler and is what this replaced: it held a
//! paragraph per page, so "full-text" search read the first paragraph, and the
//! whole file was fetched on the first keystroke. Here the build writes
//!
//! - `search/manifest.json` — the chunk ids, nothing else to guess from;
//! - `search/t-<id>.json` — the postings for every term starting `<id>`;
//! - `search/m-<n>.json` — the card (title, address, section, description) for
//!   pages `n*64 .. n*64+64`: what a result list needs, and little of it;
//! - `search/x-<page>.json` — one page's text, for the matched line under a
//!   result. Only the dozen pages actually shown are ever fetched, so the
//!   text — by far the largest part of any index — is never transferred for a
//!   page nobody is looking at.
//!
//! The page tokenises a query the same way this does, asks the manifest which
//! chunk a term lives in, and fetches that one. A chunk id is never built from
//! what the reader typed: the page matches the typed term against the ids the
//! manifest lists, and an id that is not listed is simply not fetched.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::docs::DocPage;
use super::docs_text::plain_text_from_markdown;

/// Pages per metadata chunk. One chunk is what a page of results costs.
const PAGES_PER_META_CHUNK: usize = 64;
/// A term chunk is split by a longer prefix once it holds more terms than this.
const MAX_TERMS_PER_CHUNK: usize = 400;
/// How much of a page's text the snippet is drawn from.
const SNIPPET_SOURCE_CHARS: usize = 1_200;
/// Pages kept per term, best first: a term in every page of a large site is
/// not a search result, it is a word.
const MAX_POSTINGS_PER_TERM: usize = 500;

/// Where a term was seen, and what that is worth.
const WEIGHT_TITLE: u32 = 12;
const WEIGHT_KEYWORD: u32 = 8;
const WEIGHT_HEADING: u32 = 6;
const WEIGHT_DESCRIPTION: u32 = 4;
const WEIGHT_BODY: u32 = 1;

/// One file of the built index.
#[derive(Debug, Clone)]
pub struct SearchFile {
    /// Path relative to the site root, e.g. `search/t-en.json`.
    pub rel_path: String,
    pub contents: String,
}

/// Lower case, split on everything that is not a letter or a digit, two
/// characters or more, thirty-two at most. The page's tokeniser is the same
/// function in JavaScript; the two must not drift, or a word indexed here is
/// unfindable there.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            push_token(&mut out, std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        push_token(&mut out, current);
    }
    out
}

fn push_token(out: &mut Vec<String>, token: String) {
    if token.chars().count() >= 2 && token.chars().count() <= 32 {
        out.push(token);
    }
}

/// The chunk a term belongs to: its first character, or its first two once
/// that letter holds too many terms to be worth fetching whole.
fn short_prefix(term: &str) -> String {
    term.chars().take(1).collect()
}

fn long_prefix(term: &str) -> String {
    term.chars().take(2).collect()
}

/// Builds every file of the index for one site, in reading order.
pub fn build(pages: &[DocPage]) -> Vec<SearchFile> {
    // term -> page index -> weight
    let mut postings: BTreeMap<String, BTreeMap<usize, u32>> = BTreeMap::new();
    let add = |term: String, page: usize, weight: u32, postings: &mut BTreeMap<String, BTreeMap<usize, u32>>| {
        let entry = postings.entry(term).or_default().entry(page).or_insert(0);
        *entry = entry.saturating_add(weight);
    };

    for (index, page) in pages.iter().enumerate() {
        if page.noindex {
            continue;
        }
        for term in tokenize(&page.title) {
            add(term, index, WEIGHT_TITLE, &mut postings);
        }
        for keyword in &page.keywords {
            for term in tokenize(keyword) {
                add(term, index, WEIGHT_KEYWORD, &mut postings);
            }
        }
        for heading in &page.headings {
            for term in tokenize(&heading.text) {
                add(term, index, WEIGHT_HEADING, &mut postings);
            }
        }
        for term in tokenize(&page.description) {
            add(term, index, WEIGHT_DESCRIPTION, &mut postings);
        }
        for term in tokenize(&plain_text_from_markdown(&page.markdown)) {
            add(term, index, WEIGHT_BODY, &mut postings);
        }
    }

    // Which prefix length each first character needs.
    let mut per_letter: BTreeMap<String, usize> = BTreeMap::new();
    for term in postings.keys() {
        *per_letter.entry(short_prefix(term)).or_insert(0) += 1;
    }

    let mut chunks: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
    for (term, pages_for_term) in &postings {
        let letter = short_prefix(term);
        let id = if per_letter.get(&letter).copied().unwrap_or(0) > MAX_TERMS_PER_CHUNK {
            long_prefix(term)
        } else {
            letter
        };
        let mut entries = pages_for_term.iter().map(|(page, weight)| (*page, *weight)).collect::<Vec<_>>();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        entries.truncate(MAX_POSTINGS_PER_TERM);
        chunks.entry(id).or_default().insert(
            term.clone(),
            Value::Array(entries.into_iter().map(|(page, weight)| json!([page, weight])).collect()),
        );
    }

    let mut files = Vec::new();
    for (id, terms) in &chunks {
        files.push(SearchFile {
            rel_path: format!("search/t-{id}.json"),
            contents: serde_json::to_string(terms).unwrap_or_else(|_| "{}".to_string()),
        });
    }

    // Metadata, in blocks of pages, so showing ten results costs one or two
    // fetches however large the site is.
    let mut meta_chunks = 0usize;
    for (block, group) in pages.chunks(PAGES_PER_META_CHUNK).enumerate() {
        let mut entries = serde_json::Map::new();
        for (offset, page) in group.iter().enumerate() {
            if page.noindex {
                continue;
            }
            let index = block * PAGES_PER_META_CHUNK + offset;
            entries.insert(
                index.to_string(),
                json!({
                    "title": page.title,
                    "href": page.route_path,
                    "section": section_label(page),
                    "description": page.description,
                }),
            );
            // The page's text, on its own, fetched only if this page is shown.
            files.push(SearchFile {
                rel_path: format!("search/x-{index}.json"),
                contents: json!({ "text": snippet_source(&page.markdown) }).to_string(),
            });
        }
        if entries.is_empty() {
            continue;
        }
        meta_chunks = meta_chunks.max(block + 1);
        files.push(SearchFile {
            rel_path: format!("search/m-{block}.json"),
            contents: Value::Object(entries).to_string(),
        });
    }

    let manifest = json!({
        "version": 1,
        "pages": pages.iter().filter(|page| !page.noindex).count(),
        "terms": postings.len(),
        "term_chunks": chunks.keys().cloned().collect::<Vec<_>>(),
        "meta_chunks": meta_chunks,
        "pages_per_meta_chunk": PAGES_PER_META_CHUNK,
    });
    files.push(SearchFile {
        rel_path: manifest_rel_path(),
        contents: serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string()),
    });
    files
}

pub fn manifest_rel_path() -> String {
    "search/manifest.json".to_string()
}

/// The folder path a page reads the index from, inside the served site.
pub fn manifest_route(deploy_base_path: &str) -> String {
    format!("{}search/manifest.json", if deploy_base_path.ends_with('/') { deploy_base_path.to_string() } else { format!("{deploy_base_path}/") })
}

fn section_label(page: &DocPage) -> String {
    if page.slug_segments.len() > 1 {
        page.slug_segments[..page.slug_segments.len() - 1]
            .iter()
            .map(|segment| super::docs_text::titleize_segment(segment))
            .collect::<Vec<_>>()
            .join(" / ")
    } else {
        String::new()
    }
}

fn snippet_source(markdown: &str) -> String {
    let text = plain_text_from_markdown(markdown);
    let mut out = String::with_capacity(SNIPPET_SOURCE_CHARS.min(text.len()));
    for (count, ch) in text.chars().enumerate() {
        if count >= SNIPPET_SOURCE_CHARS {
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(title: &str, slug: &str, markdown: &str) -> DocPage {
        DocPage {
            slug_segments: vec![slug.to_string()],
            title: title.to_string(),
            description: String::new(),
            keywords: Vec::new(),
            canonical: None,
            noindex: false,
            blocks: super::super::docs_blocks::parse(markdown),
            markdown: markdown.to_string(),
            headings: Vec::new(),
            source_rel_path: format!("{slug}.md"),
            output_rel_path: format!("{slug}/index.html"),
            route_path: format!("/{slug}/"),
            order: None,
            source_modified: None,
        }
    }

    /// A word in the body is findable — the whole reason the flat index was
    /// replaced — and it is in the chunk its first letter names.
    #[test]
    fn a_word_in_the_body_is_in_the_index() {
        let pages = vec![
            page("Bell inequalities", "bell", "The singlet state always disagrees."),
            page("Decoherence", "decoherence", "The environment records which branch."),
        ];
        let files = build(&pages);
        let chunk = files
            .iter()
            .find(|file| file.rel_path == "search/t-s.json")
            .expect("a chunk for terms starting with s");
        let terms: serde_json::Value = serde_json::from_str(&chunk.contents).expect("json");
        assert!(terms.get("singlet").is_some(), "{}", chunk.contents);
        assert_eq!(terms["singlet"][0][0], 0, "the posting names the first page");

        let manifest = files.iter().find(|file| file.rel_path == manifest_rel_path()).expect("manifest");
        let manifest: serde_json::Value = serde_json::from_str(&manifest.contents).expect("json");
        assert_eq!(manifest["pages"], 2);
        assert!(
            manifest["term_chunks"].as_array().expect("chunks").iter().any(|id| id == "s"),
            "the manifest lists every chunk the page may fetch"
        );
    }

    /// The title is worth more than the body, so the page named for a word
    /// outranks the page that merely mentions it.
    #[test]
    fn a_title_outweighs_a_mention() {
        let pages = vec![
            page("Decoherence", "decoherence", "What the environment records."),
            page("Entanglement", "entanglement", "Decoherence is covered later."),
        ];
        let files = build(&pages);
        let chunk = files.iter().find(|file| file.rel_path == "search/t-d.json").expect("chunk");
        let terms: serde_json::Value = serde_json::from_str(&chunk.contents).expect("json");
        let postings = terms["decoherence"].as_array().expect("postings");
        assert_eq!(postings[0][0], 0, "the page titled for the term comes first");
        assert!(postings[0][1].as_u64() > postings[1][1].as_u64());
    }

    /// A `noindex` page is in no chunk and no metadata block.
    #[test]
    fn a_noindex_page_is_not_in_the_index() {
        let mut hidden = page("Draft", "draft", "Secret word: xylophone.");
        hidden.noindex = true;
        let files = build(&[hidden]);
        assert!(files.iter().all(|file| !file.contents.contains("xylophone")), "nothing of it is written");
        let manifest = files.iter().find(|file| file.rel_path == manifest_rel_path()).expect("manifest");
        assert!(manifest.contents.contains("\"pages\":0"));
    }
}
