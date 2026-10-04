//! "Did you mean" for a name the catalogue or the payload does not have.
//!
//! One edit distance for every suggestion — a node id, an answer key, a
//! kind — so a typo is judged the same way everywhere. A kind is also found
//! by its words: `static.page.generate` names no kind, but its words are in
//! `web.site.generate`'s title and description.

use std::collections::BTreeSet;

use crate::pipeline::NodeDefinition;

/// Edits between two names, a swap of two neighbours counting as one
/// (`fecth` is one edit from `fetch`).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=b.len() {
        d[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// The closest name within a small edit distance (a third of its length,
/// at least one), if any.
pub fn did_you_mean<'a>(wanted: &str, names: &[&'a str]) -> Option<&'a str> {
    let limit = (wanted.chars().count() / 3).max(1);
    names
        .iter()
        .map(|name| (edit_distance(wanted, name), *name))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, name)| name)
}

/// Lowercase words of a text, a trailing plural `s` dropped (`pages` is
/// `page`), one-letter words skipped (a family may be two: `fs`, `ai`).
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 2)
        .map(|w| {
            let w = w.to_ascii_lowercase();
            match w.strip_suffix('s') {
                Some(stem) if stem.len() >= 3 && !stem.ends_with('s') => stem.to_string(),
                _ => w,
            }
        })
        .collect()
}

/// Up to `max` kinds a mistyped or invented kind most likely meant: first
/// the ones a few edits away (a typo), then the ones sharing the most of
/// its words — a segment of the kind counts twice, a word of the title or
/// description once — with fewer edits breaking a tie.
pub fn suggest_kinds(wanted: &str, defs: &[NodeDefinition], max: usize) -> Vec<String> {
    let wanted = wanted.trim().to_ascii_lowercase();
    // The retired `n.` prefix names nothing; the words after it do.
    let wanted = wanted.strip_prefix("n.").map(str::to_string).unwrap_or(wanted);
    if wanted.is_empty() {
        return Vec::new();
    }
    let asked = words(&wanted);
    let typo_limit = (wanted.chars().count() / 4).max(2);
    let mut scored: Vec<(bool, usize, usize, &str)> = defs
        .iter()
        .map(|def| {
            let distance = edit_distance(&wanted, &def.kind);
            let segments = words(&def.kind.replace('.', " "));
            let prose = words(&format!("{} {}", def.title, def.description));
            let score: usize = asked
                .iter()
                .map(|w| 2 * usize::from(segments.contains(w)) + usize::from(prose.contains(w)))
                .sum();
            (distance <= typo_limit, score, distance, def.kind.as_str())
        })
        .filter(|(typo, score, _, _)| *typo || *score > 0)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(b.3)));
    scored.into_iter().take(max).map(|(_, _, _, kind)| kind.to_string()).collect()
}

/// `did you mean `a`, `b`?` for a refusal, or nothing.
pub fn did_you_mean_phrase(candidates: &[String]) -> String {
    if candidates.is_empty() {
        return String::new();
    }
    let listed: Vec<String> = candidates.iter().map(|c| format!("`{c}`")).collect();
    format!("; did you mean {}?", listed.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs() -> Vec<NodeDefinition> {
        crate::pipeline::nodes::builtin_node_definitions()
    }

    #[test]
    fn a_swap_of_neighbours_is_one_edit() {
        assert_eq!(edit_distance("fecth", "fetch"), 1);
        assert_eq!(did_you_mean("scirpt", &["script", "query"]), Some("script"));
        assert_eq!(did_you_mean("result", &["script"]), None);
    }

    #[test]
    fn a_typo_finds_its_kind_first() {
        let found = suggest_kinds("fs.image.thumbnial", &defs(), 3);
        assert_eq!(found.first().map(String::as_str), Some("fs.image.thumbnail"), "{found:?}");
    }

    #[test]
    fn an_invented_kind_finds_the_one_its_words_describe() {
        for wanted in ["static.page.generate", "web.page.build"] {
            let found = suggest_kinds(wanted, &defs(), 3);
            assert!(found.iter().any(|k| k == "web.site.generate"), "{wanted}: {found:?}");
        }
    }
}
