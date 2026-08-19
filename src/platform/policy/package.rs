//! Hub package policy scanner.
//!
//! This module owns the reusable package safety review used before publishing
//! or adding Hub packages. It deliberately returns facts instead of UI text
//! layout so Project Studio, CLI, and future APIs can render the same decision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::report::PolicyRiskLevel;

const LARGE_FILE_THRESHOLD_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePolicyEntry {
    pub rel_path: String,
    pub kind: String,
    pub size_bytes: usize,
    #[serde(default)]
    pub content: String,
    /// Why the review could not read this entry's bytes, when it could not.
    ///
    /// Empty `content` means an empty file. This means the reviewer was handed
    /// nothing while the install still writes something, which is the opposite
    /// finding and must never produce the same verdict.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unreadable: String,
}

impl PackagePolicyEntry {
    pub fn text(&self) -> Option<&str> {
        if self.content.is_empty() {
            None
        } else {
            Some(&self.content)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageSafetyReview {
    pub nodes_used: Vec<String>,
    pub credentials_required: Vec<String>,
    pub external_urls: Vec<String>,
    pub database_effects: Vec<String>,
    pub filesystem_effects: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    /// What the install-time SQL does, one row per file that would be replayed.
    #[serde(default)]
    pub database_initialization: Vec<DatabaseInitializationReport>,
    /// Effects the installer reports and the user may accept.
    pub warnings: Vec<String>,
    /// Findings that make a package uninstallable.
    ///
    /// These are never overridable. A warning asks whether the user accepts an
    /// effect; a violation says the package will not be installed at all,
    /// whoever approves it. The contract validator already refuses malformed
    /// bundles; this refuses well-formed ones whose behaviour is unsafe.
    #[serde(default)]
    pub violations: Vec<String>,
    /// `low`, `medium`, `high`, or `blocked` when violations are present.
    pub risk_level: String,
}

/// What one initial-data file would do, and to which store.
///
/// This is a reading of the SQL the installer would replay, statement by
/// statement, so a reviewer learns what the package does to their data without
/// opening the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseInitializationReport {
    /// `sekejap` or `sqlite`: the only engines an install replays SQL into.
    pub engine: String,
    /// The package path this SQL comes from.
    pub source: String,
    /// Which store receives it, in the reader's terms rather than the engine's.
    pub store: String,
    /// Whether a database the user already had is exposed to this SQL.
    pub existing_data_at_risk: bool,
    /// How many statements of each kind, including the `OTHER` this reader did
    /// not recognise. The counts always sum to the number of statements the
    /// installer would run.
    pub statements: BTreeMap<String, usize>,
    /// Every table the statements name, sorted.
    pub tables: Vec<String>,
    /// Statements that drop, delete, truncate, or alter, quoted back.
    pub destructive: Vec<String>,
    /// Why this file has no statement breakdown, when its bytes could not be
    /// read. Empty means the breakdown above is the whole file.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unreadable: String,
}

#[derive(Debug, Clone, Default)]
pub struct PackageReviewOptions {
    pub publish_mode: bool,
    pub require_title: bool,
    pub require_description: bool,
    pub require_cover_image: bool,
    pub title: String,
    pub description: String,
    pub has_cover_image: bool,
}

pub fn review_package_entries(
    entries: &[PackagePolicyEntry],
    mut warnings: Vec<String>,
    options: PackageReviewOptions,
) -> PackageSafetyReview {
    let mut nodes_used = BTreeSet::new();
    let mut credentials_required = BTreeSet::new();
    let mut external_urls = BTreeSet::new();
    let mut database_effects = BTreeSet::new();
    let mut filesystem_effects = BTreeSet::new();
    let mut public_endpoints = BTreeSet::new();
    let mut schedules = BTreeSet::new();
    let mut large_files = Vec::new();
    let mut seed_data = Vec::new();
    let mut database_initialization = Vec::new();
    let mut violations = Vec::new();

    if options.require_title && options.title.trim().is_empty() {
        warnings.push("package title is empty".to_string());
    }
    if options.require_description && options.description.trim().is_empty() {
        warnings.push("package description is empty".to_string());
    }
    if options.require_cover_image && !options.has_cover_image {
        warnings.push("package has no cover image".to_string());
    }

    for entry in entries {
        if is_reviewed_pipeline_path(&entry.rel_path) {
            if !entry.unreadable.is_empty() {
                // The destination decides what is scanned, so an entry that
                // lands here is a pipeline whatever supplied its bytes. Bytes
                // the review cannot read are bytes the install would still
                // write, which is a refusal and not a clean result.
                violations.push(format!(
                    "{}: a pipeline the safety review cannot read ({})",
                    entry.rel_path, entry.unreadable
                ));
            } else if let Some(source) = entry.text() {
                analyze_pipeline_text(
                    source,
                    &mut nodes_used,
                    &mut credentials_required,
                    &mut external_urls,
                    &mut database_effects,
                    &mut filesystem_effects,
                    &mut public_endpoints,
                    &mut schedules,
                    &mut warnings,
                );
            }
        } else if options.publish_mode {
            if let Some(source) = entry.text() {
                collect_urls_from_text(source, &mut external_urls);
            }
        }

        if entry.size_bytes >= LARGE_FILE_THRESHOLD_BYTES {
            large_files.push(format!("{} ({})", entry.rel_path, entry.size_bytes));
        }
        if looks_like_seed_data(&entry.rel_path, entry.size_bytes) {
            seed_data.push(entry.rel_path.clone());
        }
        if let Some(engine) = initial_data_engine_for_rel_path(&entry.rel_path) {
            let mut report =
                review_initial_data_sql(engine, &entry.rel_path, entry.text().unwrap_or_default());
            if !entry.unreadable.is_empty() {
                // A seed file whose bytes the review never saw is still a seed
                // file the install would replay. It keeps its row, saying why
                // the row is empty, so a missing breakdown cannot be read as
                // "this file does nothing".
                report.unreadable = entry.unreadable.clone();
                warnings.push(format!(
                    "{}: initial data the safety review cannot read ({})",
                    entry.rel_path, entry.unreadable
                ));
            }
            database_initialization.push(report);
        }
        if options.publish_mode && looks_like_private_publish_path(&entry.rel_path) {
            warnings.push(format!(
                "{} looks like private/runtime data",
                entry.rel_path
            ));
        }
    }

    database_initialization.sort_by(|a, b| a.source.cmp(&b.source));
    warnings.sort();
    warnings.dedup();
    violations.sort();
    violations.dedup();

    let mut risk_score = 0;
    if !credentials_required.is_empty() {
        risk_score += 1;
    }
    if !external_urls.is_empty() {
        risk_score += 1;
    }
    if !database_effects.is_empty() {
        risk_score += 2;
    }
    if !filesystem_effects.is_empty() {
        risk_score += 1;
    }
    if !public_endpoints.is_empty() || !schedules.is_empty() {
        risk_score += 2;
    }
    if !large_files.is_empty() || !seed_data.is_empty() {
        risk_score += 1;
    }
    if options.publish_mode && !warnings.is_empty() {
        risk_score += 1;
    }

    PackageSafetyReview {
        nodes_used: nodes_used.into_iter().collect(),
        credentials_required: credentials_required.into_iter().collect(),
        external_urls: external_urls.into_iter().collect(),
        database_effects: database_effects.into_iter().collect(),
        filesystem_effects: filesystem_effects.into_iter().collect(),
        public_endpoints: public_endpoints.into_iter().collect(),
        schedules: schedules.into_iter().collect(),
        large_files,
        seed_data,
        database_initialization,
        warnings,
        // A violation outranks every score: no arrangement of effects can make
        // a package that cannot be reviewed safe to install.
        risk_level: if violations.is_empty() {
            PolicyRiskLevel::from_score(risk_score)
        } else {
            PolicyRiskLevel::Blocked
        }
        .as_str()
        .to_string(),
        violations,
    }
}

/// Whether this destination is reviewed as a pipeline definition.
///
/// The path is the whole test, and it is applied to the path the install
/// actually writes, so the review and the install never disagree about which
/// entries are pipelines.
fn is_reviewed_pipeline_path(rel_path: &str) -> bool {
    rel_path.ends_with(".zf.json") && rel_path.starts_with("pipelines/")
}

impl PackageSafetyReview {
    /// Whether this package may be installed at all.
    ///
    /// Violations are never overridable, so this is checked before any approval
    /// the caller supplies is considered.
    pub fn is_installable(&self) -> bool {
        self.violations.is_empty()
    }
}

fn looks_like_seed_data(rel_path: &str, size_bytes: usize) -> bool {
    let path = rel_path.to_ascii_lowercase();
    if path.contains("/seed") || path.contains("/sample") || path.contains("/demo") {
        return true;
    }
    matches!(
        Path::new(rel_path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "csv" | "jsonl" | "sqlite" | "db" | "parquet"
    ) && size_bytes > 0
}

fn looks_like_private_publish_path(path: &str) -> bool {
    let lower = path.trim().to_ascii_lowercase();
    lower.starts_with(".git/")
        || lower.starts_with(".env")
        || lower.contains("/.env")
        || lower.contains("secret")
        || lower.contains("credential")
        || lower.starts_with("data/")
        || lower.starts_with("runtime/")
        || lower.starts_with("target/")
        || lower.ends_with(".db")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite3")
}

#[allow(clippy::too_many_arguments)]
fn analyze_pipeline_text(
    source: &str,
    nodes_used: &mut BTreeSet<String>,
    credentials_required: &mut BTreeSet<String>,
    external_urls: &mut BTreeSet<String>,
    database_effects: &mut BTreeSet<String>,
    filesystem_effects: &mut BTreeSet<String>,
    public_endpoints: &mut BTreeSet<String>,
    schedules: &mut BTreeSet<String>,
    warnings: &mut Vec<String>,
) {
    let Ok(value) = serde_json::from_str::<Value>(source) else {
        warnings.push("One pipeline could not be parsed for safety review".to_string());
        return;
    };
    analyze_pipeline_value(
        &value,
        nodes_used,
        credentials_required,
        external_urls,
        database_effects,
        filesystem_effects,
        public_endpoints,
        schedules,
    );
}

#[allow(clippy::too_many_arguments)]
fn analyze_pipeline_value(
    value: &Value,
    nodes_used: &mut BTreeSet<String>,
    credentials_required: &mut BTreeSet<String>,
    external_urls: &mut BTreeSet<String>,
    database_effects: &mut BTreeSet<String>,
    filesystem_effects: &mut BTreeSet<String>,
    public_endpoints: &mut BTreeSet<String>,
    schedules: &mut BTreeSet<String>,
) {
    match value {
        Value::Object(map) => {
            let kind = map
                .get("kind")
                .and_then(Value::as_str)
                .or_else(|| map.get("type").and_then(Value::as_str))
                .unwrap_or_default();
            if !kind.is_empty() {
                nodes_used.insert(kind.to_string());
                let kind_lc = kind.to_ascii_lowercase();
                if kind_lc.contains("pg")
                    || kind_lc.contains("mysql")
                    || kind_lc.contains("sqlite")
                    || kind_lc.contains("sekejap")
                    || kind_lc.contains("table.")
                    || kind_lc.contains("db.")
                {
                    database_effects.insert(kind.to_string());
                }
                if kind_lc.contains("fs.") || kind_lc.contains("file") {
                    filesystem_effects.insert(kind.to_string());
                }
                if kind_lc.contains("trigger.webhook") {
                    public_endpoints.insert("webhook trigger".to_string());
                }
                if kind_lc.contains("trigger.schedule") {
                    schedules.insert("schedule trigger".to_string());
                }
            }
            for (key, child) in map {
                let key_lc = key.to_ascii_lowercase();
                if key_lc.contains("credential") {
                    if let Some(s) = child.as_str().filter(|s| !s.trim().is_empty()) {
                        credentials_required.insert(s.trim().to_string());
                    }
                }
                if matches!(key_lc.as_str(), "url" | "base_url" | "endpoint" | "host") {
                    if let Some(s) = child.as_str() {
                        collect_urls_from_text(s, external_urls);
                    }
                }
                if matches!(key_lc.as_str(), "path" | "route") {
                    if let Some(s) = child.as_str().filter(|s| s.starts_with('/')) {
                        public_endpoints.insert(s.to_string());
                    }
                }
                if key_lc.contains("cron") || key_lc.contains("schedule") {
                    if let Some(s) = child.as_str().filter(|s| !s.trim().is_empty()) {
                        schedules.insert(s.trim().to_string());
                    }
                }
                analyze_pipeline_value(
                    child,
                    nodes_used,
                    credentials_required,
                    external_urls,
                    database_effects,
                    filesystem_effects,
                    public_endpoints,
                    schedules,
                );
            }
        }
        Value::Array(items) => {
            for item in items {
                analyze_pipeline_value(
                    item,
                    nodes_used,
                    credentials_required,
                    external_urls,
                    database_effects,
                    filesystem_effects,
                    public_endpoints,
                    schedules,
                );
            }
        }
        Value::String(text) => collect_urls_from_text(text, external_urls),
        _ => {}
    }
}

fn collect_urls_from_text(text: &str, out: &mut BTreeSet<String>) {
    for part in text.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | ')' | '(' | ','))
    {
        let trimmed = part
            .trim_matches(|c: char| matches!(c, '"' | '\'' | ')' | '(' | ',' | ';'))
            .trim();
        if trimmed.starts_with("https://") || trimmed.starts_with("http://") {
            out.insert(trimmed.to_string());
        }
    }
}

// ── Database initialization ─────────────────────────────────────────────
//
// A package may carry SQL that the install replays. The reader below is
// deliberately small: it recognises the head of a statement and nothing else,
// because a full SQL grammar would be a dependency, a parse failure mode, and a
// second opinion about statements the installer is going to run either way.

/// Where a package's install-time SQL lives, and which engine replays it.
///
/// The list is short on purpose. Both engines are project-local stores the
/// install creates for itself, so bundle SQL has no route to a database the
/// user already had -- there is no postgres or mysql seed execution. Adding an
/// engine here means answering [`initial_data_store_note`] for it first.
pub const INITIAL_DATA_DIRS: &[(&str, &str)] = &[
    ("initial-data/sekejap", "sekejap"),
    ("initial-data/sqlite", "sqlite"),
    ("init/sekejap", "sekejap"),
    ("init/sqlite", "sqlite"),
    ("seeds/sekejap", "sekejap"),
    ("seeds/sqlite", "sqlite"),
];

/// The engine that would replay `rel_path`, when the installer would replay it.
///
/// The prefix is anchored, so this names the files an install actually runs and
/// not every `.sql` the package happens to ship.
pub fn initial_data_engine_for_rel_path(rel_path: &str) -> Option<&'static str> {
    let rel = rel_path
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("./");
    if !rel.ends_with(".sql") {
        return None;
    }
    INITIAL_DATA_DIRS
        .iter()
        .find_map(|(prefix, engine)| rel.starts_with(&format!("{prefix}/")).then_some(*engine))
}

/// Splits an initial-data file into the statements the installer replays.
///
/// This is the installer's own splitter rather than a second opinion about the
/// file, so "96 inserts" in a report is 96 executions and not an estimate.
pub fn split_initial_data_sql(sql: &str) -> Vec<String> {
    let uncommented = sql
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    uncommented
        .split(';')
        .map(str::trim)
        .filter(|stmt| !stmt.is_empty())
        .map(|stmt| format!("{stmt};"))
        .collect()
}

/// The count key for a statement whose head this reader does not recognise.
///
/// Unrecognised is a finding, not an absence: a statement that vanished from
/// the totals would be a statement nobody was told about.
const OTHER_STATEMENT_KIND: &str = "OTHER";

/// How much of a destructive statement is quoted back before it is cut.
const DESTRUCTIVE_QUOTE_CHARS: usize = 160;

/// What one initial-data file will do, read from its own SQL.
pub fn review_initial_data_sql(
    engine: &str,
    source: &str,
    sql: &str,
) -> DatabaseInitializationReport {
    let (store, existing_data_at_risk) = initial_data_store_note(engine);
    let mut statements: BTreeMap<String, usize> = BTreeMap::new();
    let mut tables = BTreeSet::new();
    let mut destructive = Vec::new();
    for statement in split_initial_data_sql(sql) {
        let facts = classify_sql_statement(&statement);
        *statements.entry(facts.kind.to_string()).or_insert(0) += 1;
        if let Some(table) = facts.table {
            tables.insert(table);
        }
        if facts.destructive {
            destructive.push(quote_sql_statement(&statement));
        }
    }
    DatabaseInitializationReport {
        engine: engine.to_string(),
        source: source.to_string(),
        store: store.to_string(),
        existing_data_at_risk,
        statements,
        tables: tables.into_iter().collect(),
        destructive,
        unreadable: String::new(),
    }
}

/// Which store an engine writes to, and whether existing data is exposed to it.
///
/// Both supported engines are project-local stores the install creates for
/// itself. That is the most reassuring thing this report can say, so it is said
/// rather than implied. An engine this build does not know is reported as a
/// risk, because a store nobody described is a store nobody checked.
fn initial_data_store_note(engine: &str) -> (&'static str, bool) {
    match engine {
        "sekejap" | "sqlite" => ("created by this install", false),
        _ => ("not described by this build", true),
    }
}

/// What one statement is and what it touches.
struct SqlStatementFacts {
    kind: &'static str,
    table: Option<String>,
    destructive: bool,
}

fn classify_sql_statement(statement: &str) -> SqlStatementFacts {
    let body = strip_leading_sql_noise(statement);
    let Some((verb, rest)) = take_keyword(body) else {
        return SqlStatementFacts {
            kind: OTHER_STATEMENT_KIND,
            table: None,
            destructive: false,
        };
    };
    // The verb alone decides destructiveness. `ALTER VIEW` is not a kind this
    // report names, but a reader should not have to learn about it from
    // silence, so it is still flagged.
    let destructive = matches!(verb.as_str(), "DROP" | "DELETE" | "TRUNCATE" | "ALTER");
    let (kind, table) = match verb.as_str() {
        "INSERT" => keyword_then_identifier(rest, "INTO", "INSERT INTO"),
        "ALTER" => keyword_then_identifier(rest, "TABLE", "ALTER TABLE"),
        "DELETE" => keyword_then_identifier(rest, "FROM", "DELETE FROM"),
        "DROP" => keyword_then_identifier(rest, "TABLE", "DROP TABLE"),
        "UPDATE" => ("UPDATE", read_identifier(rest)),
        "TRUNCATE" => {
            let rest = take_expected(rest, "TABLE").unwrap_or(rest);
            ("TRUNCATE", read_identifier(rest))
        }
        "CREATE" => classify_create_statement(rest),
        _ => (OTHER_STATEMENT_KIND, None),
    };
    SqlStatementFacts {
        kind,
        table,
        destructive,
    }
}

/// `<keyword> [IF [NOT] EXISTS] <identifier>`, or `OTHER` when the statement
/// does not continue the way its verb promised.
fn keyword_then_identifier(
    rest: &str,
    keyword: &str,
    kind: &'static str,
) -> (&'static str, Option<String>) {
    match take_expected(rest, keyword) {
        Some(rest) => (kind, read_identifier(skip_existence_clause(rest))),
        None => (OTHER_STATEMENT_KIND, None),
    }
}

fn classify_create_statement(rest: &str) -> (&'static str, Option<String>) {
    // `CREATE UNIQUE INDEX` and `CREATE TEMP TABLE` are the same two kinds with
    // a modifier in front, so modifiers are skipped rather than counted as
    // statement kinds of their own.
    let mut rest = rest;
    while let Some((word, after)) = take_keyword(rest) {
        if matches!(
            word.as_str(),
            "UNIQUE" | "TEMP" | "TEMPORARY" | "VIRTUAL" | "OR" | "REPLACE"
        ) {
            rest = after;
            continue;
        }
        break;
    }
    let Some((word, after)) = take_keyword(rest) else {
        return (OTHER_STATEMENT_KIND, None);
    };
    match word.as_str() {
        "TABLE" => (
            "CREATE TABLE",
            read_identifier(skip_existence_clause(after)),
        ),
        "INDEX" => {
            // An index is reported against the table it indexes: its own name
            // says nothing about whose data is touched. The name is optional in
            // the dialect Sekejap accepts, so `ON` may come first.
            let after = skip_existence_clause(after);
            let after = match take_keyword(after) {
                Some((word, _)) if word == "ON" => after,
                _ => skip_identifier(after),
            };
            (
                "CREATE INDEX",
                take_expected(after, "ON").and_then(read_identifier),
            )
        }
        _ => (OTHER_STATEMENT_KIND, None),
    }
}

fn skip_existence_clause(rest: &str) -> &str {
    let Some(after_if) = take_expected(rest, "IF") else {
        return rest;
    };
    let after_not = take_expected(after_if, "NOT").unwrap_or(after_if);
    take_expected(after_not, "EXISTS").unwrap_or(rest)
}

/// Drops leading whitespace and comments, so a commented statement is still
/// classified by its verb.
fn strip_leading_sql_noise(statement: &str) -> &str {
    let mut rest = statement.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("--") {
            rest = after
                .find('\n')
                .map(|idx| &after[idx + 1..])
                .unwrap_or("")
                .trim_start();
            continue;
        }
        if let Some(after) = rest.strip_prefix("/*") {
            rest = after
                .find("*/")
                .map(|idx| &after[idx + 2..])
                .unwrap_or("")
                .trim_start();
            continue;
        }
        return rest;
    }
}

/// The next bare word, uppercased, and what follows it.
///
/// A quoted identifier deliberately does not match, so a table named `"table"`
/// is never read as the keyword.
fn take_keyword(input: &str) -> Option<(String, &str)> {
    let rest = input.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| (rest[..end].to_ascii_uppercase(), &rest[end..]))
}

fn take_expected<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let (word, rest) = take_keyword(input)?;
    (word == keyword).then_some(rest)
}

/// The identifier at the head of `input`, unquoted and unqualified, plus what
/// follows it.
fn take_identifier(input: &str) -> Option<(String, &str)> {
    let rest = input.trim_start();
    let closing = match rest.chars().next()? {
        '"' => Some('"'),
        '`' => Some('`'),
        '[' => Some(']'),
        _ => None,
    };
    if let Some(closing) = closing {
        let body = &rest[1..];
        let end = body.find(closing)?;
        let name = body[..end].trim().to_string();
        return (!name.is_empty()).then_some((name, &body[end + closing.len_utf8()..]));
    }
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '.')))
        .unwrap_or(rest.len());
    // `main.posts` names one table, and the table is what the report is about.
    let name = rest[..end]
        .trim_matches('.')
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_string();
    (!name.is_empty()).then_some((name, &rest[end..]))
}

fn read_identifier(input: &str) -> Option<String> {
    take_identifier(input).map(|(name, _)| name)
}

fn skip_identifier(input: &str) -> &str {
    take_identifier(input)
        .map(|(_, rest)| rest)
        .unwrap_or(input)
}

/// A destructive statement, quoted back on one line and cut if it runs long.
fn quote_sql_statement(statement: &str) -> String {
    let collapsed = strip_leading_sql_noise(statement)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let trimmed = collapsed.trim_end_matches(';').trim_end();
    if trimmed.chars().count() <= DESTRUCTIVE_QUOTE_CHARS {
        return trimmed.to_string();
    }
    let head = trimmed
        .chars()
        .take(DESTRUCTIVE_QUOTE_CHARS)
        .collect::<String>();
    format!("{}...", head.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A seed file as one actually arrives: comments, mixed case, quoted and
    /// qualified names, an `IF EXISTS`, and a long insert.
    const MIXED_SEED: &str = r#"
-- Blog seed data.
DROP TABLE IF EXISTS posts;
CREATE TABLE posts (_key TEXT PRIMARY KEY, slug TEXT, body TEXT);
create table if not exists `authors` (_key TEXT PRIMARY KEY, name TEXT);
CREATE UNIQUE INDEX posts_slug ON main.posts (slug);
INSERT INTO posts (_key, slug, body) VALUES ('a', 'hello', 'world');
insert into "authors" (_key, name) VALUES ('b', 'Ada');
ALTER TABLE posts ADD COLUMN tags TEXT;
UPDATE posts SET body = 'edited' WHERE _key = 'a';
DELETE FROM authors WHERE _key = 'nobody';
TRUNCATE TABLE tags;
"#;

    fn seed_entry(rel_path: &str, sql: &str) -> PackagePolicyEntry {
        PackagePolicyEntry {
            rel_path: rel_path.to_string(),
            kind: "initial data".to_string(),
            size_bytes: sql.len(),
            content: sql.to_string(),
            unreadable: String::new(),
        }
    }

    fn only_report(entries: &[PackagePolicyEntry]) -> DatabaseInitializationReport {
        let review = review_package_entries(entries, Vec::new(), PackageReviewOptions::default());
        assert_eq!(
            review.database_initialization.len(),
            1,
            "one seed file, one row"
        );
        review.database_initialization[0].clone()
    }

    /// The headline claim: a reader learns which statements run, against which
    /// tables, and which of them destroy something -- without opening the file.
    #[test]
    fn a_mixed_seed_file_reports_its_statements_tables_and_destructive_work() {
        let report = only_report(&[seed_entry("seeds/sekejap/002-posts.sql", MIXED_SEED)]);

        assert_eq!(report.engine, "sekejap");
        assert_eq!(report.source, "seeds/sekejap/002-posts.sql");
        assert_eq!(
            report.statements,
            BTreeMap::from([
                ("DROP TABLE".to_string(), 1),
                ("CREATE TABLE".to_string(), 2),
                ("CREATE INDEX".to_string(), 1),
                ("INSERT INTO".to_string(), 2),
                ("ALTER TABLE".to_string(), 1),
                ("UPDATE".to_string(), 1),
                ("DELETE FROM".to_string(), 1),
                ("TRUNCATE".to_string(), 1),
            ])
        );
        assert_eq!(report.tables, vec!["authors", "posts", "tags"]);
        assert_eq!(
            report.destructive,
            vec![
                "DROP TABLE IF EXISTS posts",
                "ALTER TABLE posts ADD COLUMN tags TEXT",
                "DELETE FROM authors WHERE _key = 'nobody'",
                "TRUNCATE TABLE tags",
            ]
        );
    }

    /// Both engines write to a store this install creates, so the report says
    /// so rather than leaving the reader to infer it.
    #[test]
    fn a_seed_file_says_no_existing_database_is_reachable() {
        for (path, engine) in [
            ("seeds/sekejap/002-posts.sql", "sekejap"),
            ("init/sqlite/001-schema.sql", "sqlite"),
        ] {
            let report = only_report(&[seed_entry(path, "INSERT INTO posts VALUES ('a');")]);
            assert_eq!(report.engine, engine);
            assert_eq!(report.store, "created by this install");
            assert!(!report.existing_data_at_risk);
        }
    }

    /// The failure mode this codebase keeps getting bitten by: a statement
    /// nobody recognised must show up in the totals, not disappear from them.
    #[test]
    fn an_unrecognised_statement_is_counted_not_dropped() {
        let sql = "INSERT INTO posts VALUES ('a');\n\
                   PRAGMA journal_mode = WAL;\n\
                   GRANT ALL ON posts TO nobody;\n\
                   VACUUM;";
        let report = only_report(&[seed_entry("seeds/sqlite/003-odd.sql", sql)]);

        assert_eq!(report.statements.get("OTHER"), Some(&3));
        assert_eq!(
            report.statements.values().sum::<usize>(),
            split_initial_data_sql(sql).len(),
            "every statement the installer runs is counted exactly once"
        );
    }

    /// A `.sql` outside the directories the installer replays is not initial
    /// data, and a non-`.sql` file inside one is not either.
    #[test]
    fn only_the_paths_the_installer_replays_are_reported() {
        for path in [
            "docs/schema.sql",
            "seeds/postgres/001.sql",
            "seeds/sekejap/notes.md",
        ] {
            assert_eq!(initial_data_engine_for_rel_path(path), None, "{path}");
        }
        assert_eq!(
            initial_data_engine_for_rel_path("initial-data/sqlite/001.sql"),
            Some("sqlite")
        );
    }

    /// A seed file the review could not read keeps its row and says why, so an
    /// empty breakdown never reads as "this file does nothing".
    #[test]
    fn a_seed_file_that_cannot_be_read_still_gets_a_row() {
        let mut entry = seed_entry("seeds/sekejap/002-posts.sql", "");
        entry.unreadable = "artifact not found on this channel".to_string();
        let review = review_package_entries(&[entry], Vec::new(), PackageReviewOptions::default());

        let report = &review.database_initialization[0];
        assert_eq!(report.unreadable, "artifact not found on this channel");
        assert!(report.statements.is_empty());
        assert!(
            review
                .warnings
                .iter()
                .any(|warning| warning.contains("seeds/sekejap/002-posts.sql")),
            "the file is named in the warnings: {:?}",
            review.warnings
        );
    }

    /// A statement long enough to bury its point is cut, and still opens with
    /// the part that says what it destroys.
    #[test]
    fn a_long_destructive_statement_is_quoted_back_and_cut() {
        let keys = (0..80)
            .map(|index| format!("'key-{index}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!("DELETE FROM posts WHERE _key IN ({keys});");
        let report = only_report(&[seed_entry("seeds/sqlite/004-purge.sql", &sql)]);

        let quoted = &report.destructive[0];
        assert!(quoted.starts_with("DELETE FROM posts WHERE _key IN ("));
        assert!(quoted.ends_with("..."));
        assert!(quoted.chars().count() <= DESTRUCTIVE_QUOTE_CHARS + 3);
    }
}
