//! Hub package policy scanner.
//!
//! This module owns the reusable package safety review used before publishing
//! or adding Hub packages. It deliberately returns facts instead of UI text
//! layout so Project Studio, CLI, and future APIs can render the same decision.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::capability::{
    derive_bundle_capabilities, known_node_capabilities, node_kinds_in_pipeline,
};
use super::report::PolicyRiskLevel;
use crate::contracts::kinds::decode_node_bundle;
use crate::pipeline::model::NodeCapability;
use crate::platform::model::{MultiNodePackageDefinition, ResolvedProjectLayout};

const LARGE_FILE_THRESHOLD_BYTES: usize = 5 * 1024 * 1024;

/// Why bytes already in hand are still not something the review can read.
const UNREADABLE_NOT_TEXT: &str = "the bytes are not UTF-8 text";

/// Conventional notice files carried by bundled libraries. This is an exact
/// name allowance, not an extension wildcard: `LICENSE.exe` remains refused.
/// Their bytes must be readable text before the ordinary extension rule is
/// bypassed, so missing artifacts and binary files cannot borrow these names.
fn is_package_notice_path(rel_path: &str) -> bool {
    matches!(
        rel_path.rsplit('/').next(),
        Some("LICENSE" | "LICENSE.marked" | "LICENSE.dompurify" | "MODIFICATIONS")
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagePolicyEntry {
    rel_path: String,
    kind: String,
    size_bytes: usize,
    #[serde(default)]
    content: String,
    /// Why the review could not read this entry's bytes, when it could not.
    ///
    /// Empty `content` means an empty file. This means the reviewer was handed
    /// nothing while the install still writes something, which is the opposite
    /// finding and must never produce the same verdict.
    ///
    /// The fields are private and the two constructors below are the only way
    /// to set them, because every caller that built this by hand got to decide
    /// for itself what "unreadable" meant -- and they did not agree.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    unreadable: String,
}

impl PackagePolicyEntry {
    /// The review's view of one entry whose bytes are in hand.
    ///
    /// This is the only place bytes become `content` or `unreadable`, so two
    /// gates cannot reach different verdicts on the same document by each
    /// deciding readability their own way. Bytes that are not text are
    /// unreadable: the reviewer was handed nothing while the install still
    /// writes something, and that is the same fact whether the bytes never
    /// arrived or arrived as binary.
    ///
    /// Being unreadable refuses nothing on its own. The scan escalates it for
    /// a pipeline or a notice that must carry readable text; ordinary binary
    /// assets such as icons and fonts still install.
    pub fn from_bytes(
        rel_path: impl Into<String>,
        kind: impl Into<String>,
        size_bytes: usize,
        bytes: &[u8],
    ) -> Self {
        let (content, unreadable) = match std::str::from_utf8(bytes) {
            Ok(text) => (text.to_string(), String::new()),
            Err(_) => (String::new(), UNREADABLE_NOT_TEXT.to_string()),
        };
        Self {
            rel_path: rel_path.into(),
            kind: kind.into(),
            size_bytes,
            content,
            unreadable,
        }
    }

    /// The review's view of an entry whose bytes never arrived at all.
    ///
    /// `reason` says what stopped them, so the refusal can name a failed fetch
    /// rather than describing it as an empty file.
    pub fn unresolved(
        rel_path: impl Into<String>,
        kind: impl Into<String>,
        size_bytes: usize,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            rel_path: rel_path.into(),
            kind: kind.into(),
            size_bytes,
            content: String::new(),
            unreadable: reason.into(),
        }
    }

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
    /// Node kinds in this package that read or write a configured store.
    ///
    /// Derived from the node catalog, which is the only thing that knows what a
    /// node does. The name predates the derivation and is kept because it is
    /// the name three templates and two API responses already use.
    pub database_effects: Vec<String>,
    /// Node kinds in this package that read or write the project's files.
    pub filesystem_effects: Vec<String>,
    /// Node kinds in this package that open an outbound connection.
    ///
    /// Distinct from `external_urls`, which lists destinations written down in
    /// a config. A node whose URL is assembled from a credential or a
    /// placeholder contributes here and nowhere else, so a package can no
    /// longer make outbound calls while reporting no destination at all.
    #[serde(default)]
    pub network_effects: Vec<String>,
    /// Node kinds in this package that run code or a program the package
    /// supplied -- a script body, a `--*-expr`, a template, a subprocess.
    ///
    /// This is the capability that carries the others with it: what a node runs
    /// can reach whatever that code can reach, which is why the review reports
    /// it separately rather than folding it into the lists above.
    #[serde(default)]
    pub code_execution: Vec<String>,
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
    /// Whether these `rel_path`s name files inside the package artifact rather
    /// than inside a project's `repo/`.
    ///
    /// A node bundle is the only package this is true of: it materializes into
    /// `data/hub/nodes/` and its file set is fixed by the `NodeBundle` contract,
    /// not by a project's layout. Applying a repository rule to it would let a
    /// project that narrowed its own extensions refuse a bundle that never
    /// touches its repository.
    ///
    /// The default is `false`, so a caller that forgets gets the repository
    /// rule enforced rather than skipped.
    pub bundle_internal_paths: bool,
}

pub fn review_package_entries(
    layout: &ResolvedProjectLayout,
    entries: &[PackagePolicyEntry],
    mut warnings: Vec<String>,
    options: PackageReviewOptions,
) -> PackageSafetyReview {
    let mut findings = PipelineFindings::default();
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

    // The node bundles this package carries, read before anything else: they
    // name their own function pipelines, which is how the review learns that a
    // file outside the repository's pipeline directory is a pipeline at all.
    let bundles = carried_node_bundles(entries);
    let mut capabilities_by_kind = known_node_capabilities().clone();
    let mut unresolved_kinds = BTreeSet::new();
    for bundle in &bundles {
        // A kind this build already provides is checked against the catalog as
        // it was before this package touched it, so a bundle cannot make its
        // own collision disappear by being merged in first.
        for node in &bundle.package.nodes {
            if capabilities_by_kind.contains_key(&node.kind) {
                violations.push(format!(
                    "{}: declares node kind '{}', which this build already provides",
                    bundle.manifest_path, node.kind
                ));
            }
        }
        let derived = derive_bundle_capabilities(
            &bundle.package,
            |rel_path| bundle.function_graph(entries, rel_path),
            &capabilities_by_kind,
        );
        capabilities_by_kind.extend(derived.nodes);
        unresolved_kinds.extend(derived.unresolved);
    }

    for entry in entries {
        if is_package_notice_path(&entry.rel_path) {
            if !entry.unreadable.is_empty()
                || entry.content.trim().is_empty()
                || entry
                    .content
                    .chars()
                    .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
            {
                violations.push(format!(
                    "{}: a package notice must contain readable, nonempty UTF-8 text without binary control characters",
                    entry.rel_path
                ));
            }
        } else if !options.bundle_internal_paths
            && let Some(refused) = layout.refused_file_type(&entry.rel_path)
        {
            violations.push(format!(
                "{}: {refused} is not a file type a package may install",
                entry.rel_path
            ));
        }
        let reviewed_as_pipeline = is_reviewed_pipeline_path(layout, &entry.rel_path)
            || bundles
                .iter()
                .any(|bundle| bundle.is_function_path(&entry.rel_path));
        if reviewed_as_pipeline {
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
                findings.read_pipeline(source, &mut warnings);
            }
        } else if options.publish_mode {
            if let Some(source) = entry.text() {
                collect_urls_from_text(source, &mut findings.external_urls);
            }
        }

        if entry.size_bytes >= LARGE_FILE_THRESHOLD_BYTES {
            large_files.push(format!("{} ({})", entry.rel_path, entry.size_bytes));
        }
        if looks_like_seed_data(&entry.rel_path, entry.size_bytes) {
            seed_data.push(entry.rel_path.clone());
        }
        if let Some(engine) = initial_data_engine_for_rel_path(layout, &entry.rel_path) {
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

    // Every node kind the package names, answered from the catalog rather than
    // from the shape of the name. A kind nothing can answer for is a finding of
    // its own: it is the review saying so, not the review staying quiet.
    let mut by_capability: BTreeMap<NodeCapability, BTreeSet<String>> = BTreeMap::new();
    for kind in &findings.nodes_used {
        match capabilities_by_kind.get(kind) {
            Some(capabilities) => {
                for capability in capabilities {
                    by_capability
                        .entry(*capability)
                        .or_default()
                        .insert(kind.clone());
                }
            }
            None => {
                unresolved_kinds.insert(kind.clone());
            }
        }
    }
    // A trigger publishes its route whether or not the route is written into
    // its config, so the kind itself is the finding. Matched exactly against
    // this build's constants rather than by substring: `contains` was fair for
    // a warning and never for the one that also drives the risk score.
    if findings
        .nodes_used
        .contains(crate::pipeline::nodes::basic::trigger::webhook::NODE_KIND)
    {
        findings
            .public_endpoints
            .insert("webhook trigger".to_string());
    }
    if findings
        .nodes_used
        .contains(crate::pipeline::nodes::basic::trigger::schedule::NODE_KIND)
    {
        findings.schedules.insert("schedule trigger".to_string());
    }

    let capability_nodes = |capability: NodeCapability| -> Vec<String> {
        by_capability
            .get(&capability)
            .map(|kinds| kinds.iter().cloned().collect())
            .unwrap_or_default()
    };
    let database_effects = capability_nodes(NodeCapability::Database);
    let filesystem_effects = capability_nodes(NodeCapability::Filesystem);
    let network_effects = capability_nodes(NodeCapability::Network);
    let code_execution = capability_nodes(NodeCapability::Process);
    let credential_nodes = capability_nodes(NodeCapability::Credential);

    if !credential_nodes.is_empty() {
        // `credentials_required` names the credentials a config already points
        // at; this names nodes that CAN reach one, which is the signal a package
        // gives when the choice is left to the installing user.
        //
        // The distinction is load-bearing and the wording carries it. A
        // capability is a ceiling -- what a kind can do in some configuration --
        // and most credential-reading nodes take one optionally, so a package
        // can list a node here while `credentials_required` stays empty. Saying
        // "reads" of a package that reads nothing is a scanner overstating what
        // it knows, and a scanner is worth exactly what its claims are worth.
        warnings.push(format!(
            "can read a stored credential, depending on how it is configured: {}",
            credential_nodes.join(", ")
        ));
    }
    if !unresolved_kinds.is_empty() {
        warnings.push(format!(
            "the safety review cannot say what these nodes do, because nothing in this package \
             provides them: {}",
            unresolved_kinds
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    warnings.sort();
    warnings.dedup();
    violations.sort();
    violations.dedup();
    database_initialization.sort_by(|a, b| a.source.cmp(&b.source));

    let mut risk_score = 0;
    if !findings.credentials_required.is_empty() || !credential_nodes.is_empty() {
        risk_score += 1;
    }
    if !findings.external_urls.is_empty() || !network_effects.is_empty() {
        risk_score += 1;
    }
    if !database_effects.is_empty() {
        risk_score += 2;
    }
    if !filesystem_effects.is_empty() {
        risk_score += 1;
    }
    if !code_execution.is_empty() {
        risk_score += 1;
    }
    if !findings.public_endpoints.is_empty() || !findings.schedules.is_empty() {
        risk_score += 2;
    }
    if !large_files.is_empty() || !seed_data.is_empty() {
        risk_score += 1;
    }
    if options.publish_mode && !warnings.is_empty() {
        risk_score += 1;
    }

    PackageSafetyReview {
        nodes_used: findings.nodes_used.into_iter().collect(),
        credentials_required: findings.credentials_required.into_iter().collect(),
        external_urls: findings.external_urls.into_iter().collect(),
        database_effects,
        filesystem_effects,
        network_effects,
        code_execution,
        public_endpoints: findings.public_endpoints.into_iter().collect(),
        schedules: findings.schedules.into_iter().collect(),
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

/// One node bundle a package carries, and where its own paths resolve from.
///
/// A bundle names its function pipelines package-relative, so the directory
/// holding `definition.json` is what turns those names into entries. That works
/// whether the bundle is the package (`definition.json` at the root) or sits
/// inside a larger one.
struct CarriedNodeBundle {
    manifest_path: String,
    root: String,
    package: MultiNodePackageDefinition,
}

impl CarriedNodeBundle {
    /// Entry path for one of this bundle's package-relative paths.
    fn entry_path(&self, rel_path: &str) -> String {
        format!("{}{}", self.root, rel_path)
    }

    fn is_function_path(&self, entry_rel_path: &str) -> bool {
        self.package
            .functions
            .values()
            .any(|rel_path| self.entry_path(rel_path) == entry_rel_path)
    }

    /// The parsed graph of one function pipeline, if the package carries it and
    /// its bytes parse.
    fn function_graph(&self, entries: &[PackagePolicyEntry], rel_path: &str) -> Option<Value> {
        let wanted = self.entry_path(rel_path);
        let entry = entries.iter().find(|entry| entry.rel_path == wanted)?;
        serde_json::from_str(entry.text()?).ok()
    }
}

/// The node bundles a package carries.
///
/// `definition.json` is the bundle contract's fixed file name, and decoding is
/// the test: an entry that decodes as a `NodeBundle` is one, and an entry that
/// does not is left alone rather than half-read.
fn carried_node_bundles(entries: &[PackagePolicyEntry]) -> Vec<CarriedNodeBundle> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .rel_path
                .rsplit('/')
                .next()
                .is_some_and(|name| name == "definition.json")
        })
        .filter_map(|entry| {
            let document = decode_node_bundle(entry.text()?.as_bytes()).ok()?;
            Some(CarriedNodeBundle {
                manifest_path: entry.rel_path.clone(),
                root: entry.rel_path[..entry.rel_path.len() - "definition.json".len()].to_string(),
                package: document.spec,
            })
        })
        .collect()
}

/// Whether this destination is reviewed as a pipeline definition.
///
/// The path is the whole test, and it is applied to the path the install
/// actually writes, so the review and the install never disagree about which
/// entries are pipelines. The layout answers, so neither can disagree with
/// discovery either.
fn is_reviewed_pipeline_path(layout: &ResolvedProjectLayout, rel_path: &str) -> bool {
    layout.is_pipeline_rel_path(rel_path)
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

/// What the review reads out of the pipelines a package carries.
///
/// These are the findings a pipeline states outright: the nodes it names, the
/// credentials and URLs written into its config, the routes it publishes and
/// the schedules it sets. What those nodes can *reach* is not here, because a
/// pipeline does not say -- that is resolved afterwards from the node catalog.
#[derive(Default)]
struct PipelineFindings {
    nodes_used: BTreeSet<String>,
    credentials_required: BTreeSet<String>,
    external_urls: BTreeSet<String>,
    public_endpoints: BTreeSet<String>,
    schedules: BTreeSet<String>,
}

impl PipelineFindings {
    fn read_pipeline(&mut self, source: &str, warnings: &mut Vec<String>) {
        let Ok(value) = serde_json::from_str::<Value>(source) else {
            warnings.push("One pipeline could not be parsed for safety review".to_string());
            return;
        };
        self.nodes_used.extend(node_kinds_in_pipeline(&value));
        self.read_value(&value);
    }

    fn read_value(&mut self, value: &Value) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let key_lc = key.to_ascii_lowercase();
                    if key_lc.contains("credential")
                        && let Some(value) = child.as_str().filter(|s| !s.trim().is_empty())
                    {
                        self.credentials_required.insert(value.trim().to_string());
                    }
                    if matches!(key_lc.as_str(), "url" | "base_url" | "endpoint" | "host")
                        && let Some(value) = child.as_str()
                    {
                        collect_urls_from_text(value, &mut self.external_urls);
                    }
                    if matches!(key_lc.as_str(), "path" | "route")
                        && let Some(value) = child.as_str().filter(|s| s.starts_with('/'))
                    {
                        self.public_endpoints.insert(value.to_string());
                    }
                    if (key_lc.contains("cron") || key_lc.contains("schedule"))
                        && let Some(value) = child.as_str().filter(|s| !s.trim().is_empty())
                    {
                        self.schedules.insert(value.trim().to_string());
                    }
                    self.read_value(child);
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.read_value(item);
                }
            }
            Value::String(text) => collect_urls_from_text(text, &mut self.external_urls),
            _ => {}
        }
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

/// Where a package's install-time SQL lives by default, and which engine
/// replays it.
///
/// This is the platform default a project inherits when it declares no layout;
/// [`ResolvedProjectLayout`] derives its own list from here, and every reader
/// goes through that rather than through this table.
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
/// Leading `/` and `./` are tolerated here because callers hand in manifest
/// paths; the anchored prefix test itself belongs to the layout.
pub fn initial_data_engine_for_rel_path<'a>(
    layout: &'a ResolvedProjectLayout,
    rel_path: &str,
) -> Option<&'a str> {
    let rel = rel_path
        .trim()
        .trim_start_matches('/')
        .trim_start_matches("./");
    layout.initial_data_engine(rel)
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
    use crate::platform::model::ZebflowJsonLayout;

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
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            entries,
            Vec::new(),
            PackageReviewOptions::default(),
        );
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
        let layout = ResolvedProjectLayout::platform_default();
        for path in [
            "docs/schema.sql",
            "seeds/postgres/001.sql",
            "seeds/sekejap/notes.md",
        ] {
            assert_eq!(
                initial_data_engine_for_rel_path(&layout, path),
                None,
                "{path}"
            );
        }
        assert_eq!(
            initial_data_engine_for_rel_path(&layout, "initial-data/sqlite/001.sql"),
            Some("sqlite")
        );
    }

    /// A seed file the review could not read keeps its row and says why, so an
    /// empty breakdown never reads as "this file does nothing".
    #[test]
    fn a_seed_file_that_cannot_be_read_still_gets_a_row() {
        let mut entry = seed_entry("seeds/sekejap/002-posts.sql", "");
        entry.unreadable = "artifact not found on this channel".to_string();
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[entry],
            Vec::new(),
            PackageReviewOptions::default(),
        );

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

    // ── The extension allowlist ────────────────────────────────────────────

    fn entry(rel_path: &str) -> PackagePolicyEntry {
        PackagePolicyEntry {
            rel_path: rel_path.to_string(),
            kind: "file".to_string(),
            size_bytes: 1,
            content: String::new(),
            unreadable: String::new(),
        }
    }

    fn review_with(layout: &ResolvedProjectLayout, paths: &[&str]) -> PackageSafetyReview {
        let entries = paths.iter().map(|path| entry(path)).collect::<Vec<_>>();
        review_package_entries(
            layout,
            &entries,
            Vec::new(),
            PackageReviewOptions::default(),
        )
    }

    /// A shell script and a shared library are the case this rule exists for.
    /// The refusal names the path and the extension, because nobody can
    /// override it and a user who cannot argue with it must at least be able to
    /// see what it objected to.
    #[test]
    fn a_package_carrying_an_executable_file_type_is_refused() {
        let review = review_with(
            &ResolvedProjectLayout::platform_default(),
            &[
                "pipelines/hub/pkg/install.sh",
                "pipelines/hub/pkg/libnode.dylib",
            ],
        );

        assert!(!review.is_installable());
        assert_eq!(review.risk_level, "blocked");
        assert_eq!(
            review.violations,
            vec![
                "pipelines/hub/pkg/install.sh: extension '.sh' is not a file type a package may \
                 install"
                    .to_string(),
                "pipelines/hub/pkg/libnode.dylib: extension '.dylib' is not a file type a package \
                 may install"
                    .to_string(),
            ]
        );
    }

    /// Everything a repository in this codebase actually holds, including the
    /// three files an install cannot do without and the `.gitkeep` the platform
    /// writes into every project itself.
    #[test]
    fn ordinary_project_content_installs_unchanged() {
        let review = review_with(
            &ResolvedProjectLayout::platform_default(),
            &[
                "pipelines/api/users-list.zf.json",
                "pipelines/pages/hello.tsx",
                "pipelines/pages/demo/deck-utils.ts",
                "pipelines/styles/main.css",
                "pipelines/assets/favicon-16x16.png",
                "pipelines/shared/ui/.gitkeep",
                "libraries/zeb/deckgl/0.1/runtime/deckgl.bundle.mjs",
                "libraries.lock.json",
                "docs/readme.md",
                "schemas/sekejap/schema.json",
                "seeds/sekejap/001-posts.sql",
                "zebflow.yaml",
                "zeb.lock",
                "zebflow.init.json",
            ],
        );

        assert!(review.violations.is_empty(), "{:?}", review.violations);
        assert!(review.is_installable());
    }

    #[test]
    fn conventional_package_notices_install_only_as_readable_text() {
        let layout = ResolvedProjectLayout::platform_default();
        let notice = b"Copyright contributors\nPermission is granted.\n";
        for name in [
            "LICENSE",
            "LICENSE.marked",
            "LICENSE.dompurify",
            "MODIFICATIONS",
        ] {
            let entries = [PackagePolicyEntry::from_bytes(
                format!("library/{name}"),
                "file",
                notice.len(),
                notice,
            )];
            for publish_mode in [false, true] {
                let review = review_package_entries(
                    &layout,
                    &entries,
                    Vec::new(),
                    PackageReviewOptions {
                        publish_mode,
                        ..Default::default()
                    },
                );
                assert!(review.is_installable(), "{name}: {:?}", review.violations);
            }
        }
        for bytes in [
            b"".as_slice(),
            b"\xff\xfe".as_slice(),
            b"text\0binary".as_slice(),
        ] {
            let review = review_package_entries(
                &layout,
                &[PackagePolicyEntry::from_bytes(
                    "LICENSE",
                    "file",
                    bytes.len(),
                    bytes,
                )],
                Vec::new(),
                PackageReviewOptions::default(),
            );
            assert!(!review.is_installable(), "a notice must be readable text");
        }
        let unresolved = review_package_entries(
            &layout,
            &[PackagePolicyEntry::unresolved(
                "LICENSE",
                "file",
                20,
                "artifact unavailable",
            )],
            Vec::new(),
            PackageReviewOptions::default(),
        );
        assert!(!unresolved.is_installable());
        for refused in [
            "LICENSE.exe",
            "LICENSE.sh",
            "LICENSE.unrecognized",
            "MODIFICATIONS.exe",
            "Makefile",
            ".env",
        ] {
            assert!(
                !review_with(&layout, &[refused]).is_installable(),
                "{refused}"
            );
        }
    }

    /// A name with no suffix is refused as an unnamed type, which is how
    /// `.env` -- a hidden file rather than an extension -- is caught.
    #[test]
    fn a_file_with_no_extension_is_refused() {
        let review = review_with(
            &ResolvedProjectLayout::platform_default(),
            &["pipelines/hub/pkg/.env", "pipelines/hub/pkg/Makefile"],
        );

        assert_eq!(
            review.violations,
            vec![
                "pipelines/hub/pkg/.env: a name with no extension is not a file type a package \
                 may install"
                    .to_string(),
                "pipelines/hub/pkg/Makefile: a name with no extension is not a file type a \
                 package may install"
                    .to_string(),
            ]
        );
    }

    /// A project may narrow the set. What it excluded is then refused for it,
    /// and the files an install cannot do without survive the narrowing --
    /// otherwise a project could make its own bundles uninstallable.
    #[test]
    fn a_project_that_narrows_its_extensions_refuses_what_it_excluded() {
        let layout = ZebflowJsonLayout {
            allowed_extensions: Some(vec!["tsx".to_string(), "json".to_string()]),
            ..ZebflowJsonLayout::default()
        }
        .resolve();

        let review = review_with(
            &layout,
            &[
                "pipelines/feed.zf.json",
                "pipelines/pages/hello.tsx",
                "pipelines/styles/main.css",
                "pipelines/assets/logo.png",
                "zebflow.yaml",
                "zeb.lock",
                "pipelines/shared/ui/.gitkeep",
            ],
        );

        assert_eq!(
            review.violations,
            vec![
                "pipelines/assets/logo.png: extension '.png' is not a file type a package may \
                 install"
                    .to_string(),
                "pipelines/styles/main.css: extension '.css' is not a file type a package may \
                 install"
                    .to_string(),
            ]
        );
    }

    // ── Capabilities, derived from the node catalog ────────────────────────

    fn pipeline_entry(rel_path: &str, kinds: &[&str]) -> PackagePolicyEntry {
        let nodes = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                serde_json::json!({"id": format!("n{index}"), "kind": kind, "config": {}})
            })
            .collect::<Vec<_>>();
        let source = serde_json::json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "Pipeline",
            "metadata": {"name": "demo"},
            "spec": {"id": "demo", "nodes": nodes, "edges": []}
        })
        .to_string();
        PackagePolicyEntry {
            rel_path: rel_path.to_string(),
            kind: "pipeline".to_string(),
            size_bytes: source.len(),
            content: source,
            unreadable: String::new(),
        }
    }

    fn bundle_entry(rel_path: &str, spec: Value) -> PackagePolicyEntry {
        let spec: crate::platform::model::MultiNodePackageDefinition =
            serde_json::from_value(spec).expect("bundle spec");
        let mut metadata = crate::contracts::ContractMetadata::named(&spec.package);
        metadata.version = Some(spec.version.clone());
        let bytes = crate::contracts::kinds::encode_node_bundle(metadata, spec)
            .expect("encode node bundle");
        PackagePolicyEntry {
            rel_path: rel_path.to_string(),
            kind: "node bundle".to_string(),
            size_bytes: bytes.len(),
            content: String::from_utf8(bytes).expect("bundle is utf-8"),
            unreadable: String::new(),
        }
    }

    /// The findings the substring matcher got right are still produced, and now
    /// for a reason: `n.pg.query` reads a store and `n.fs.put` writes files
    /// because the catalog says so, not because their names contain `pg` and
    /// `file`.
    #[test]
    fn the_effects_the_old_matcher_caught_are_still_caught() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[pipeline_entry(
                "pipelines/demo.zf.json",
                &[
                    "n.trigger.webhook",
                    "n.http.request",
                    "n.pg.query",
                    "n.fs.put",
                ],
            )],
            Vec::new(),
            PackageReviewOptions::default(),
        );

        assert!(review.database_effects.contains(&"n.pg.query".to_string()));
        assert!(review.filesystem_effects.contains(&"n.fs.put".to_string()));
        assert!(review.nodes_used.contains(&"n.http.request".to_string()));
        assert_eq!(review.risk_level, "high");
        assert!(review.violations.is_empty(), "{:?}", review.violations);
    }

    /// A trigger that publishes a route reports it even with nothing in its
    /// config to quote back, which the old matcher also got right.
    #[test]
    fn a_trigger_reports_its_route_without_a_route_written_down() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[pipeline_entry(
                "pipelines/demo.zf.json",
                &["n.trigger.webhook", "n.trigger.schedule"],
            )],
            Vec::new(),
            PackageReviewOptions::default(),
        );

        assert_eq!(review.public_endpoints, vec!["webhook trigger".to_string()]);
        assert_eq!(review.schedules, vec!["schedule trigger".to_string()]);
    }

    /// The matcher could only ever repeat the name back. These four say what
    /// the nodes reach -- including a node whose name says nothing about the
    /// network, and one whose name says `fs` while it also runs a subprocess.
    #[test]
    fn a_capability_no_name_would_reveal_is_reported() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[pipeline_entry(
                "pipelines/demo.zf.json",
                &["n.browser.run", "n.fs.compress", "n.logic.collect"],
            )],
            Vec::new(),
            PackageReviewOptions::default(),
        );

        assert_eq!(review.network_effects, vec!["n.browser.run".to_string()]);
        assert_eq!(
            review.code_execution,
            vec!["n.browser.run".to_string(), "n.fs.compress".to_string()]
        );
        // A node that reaches nothing appears in none of the effect lists, and
        // that is an answer from the catalog rather than a gap in it.
        assert!(
            !review
                .filesystem_effects
                .contains(&"n.logic.collect".to_string())
        );
        assert!(
            review
                .warnings
                .iter()
                .any(|warning| warning.starts_with("can read a stored credential")),
            "{:?}",
            review.warnings
        );
        // The wording is the assertion. A capability is a ceiling, so this
        // package lists a credential-capable node while requiring no credential
        // at all; a warning saying it "reads" one would be the scanner claiming
        // more than it knows.
        assert!(
            review.credentials_required.is_empty(),
            "no credential is configured, so none may be reported as required: {:?}",
            review.credentials_required
        );
    }

    /// A kind nothing provides is named in the warnings rather than passed over.
    /// It is a warning and not a refusal: the node may well be installed here
    /// already, and a package is not wrong for depending on one.
    #[test]
    fn a_node_kind_this_package_cannot_account_for_is_reported() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[pipeline_entry(
                "pipelines/demo.zf.json",
                &["n.x.nowhere.thing", "n.fs.put"],
            )],
            Vec::new(),
            PackageReviewOptions::default(),
        );

        assert!(
            review
                .warnings
                .iter()
                .any(|warning| warning.contains("n.x.nowhere.thing")),
            "{:?}",
            review.warnings
        );
        assert!(review.is_installable());
    }

    /// A bundle's own function pipelines are reviewed, so a node bundle stops
    /// being a package the review had nothing to say about. The declared node
    /// then carries what its function composes.
    #[test]
    fn a_node_bundles_function_pipelines_are_reviewed_like_any_other() {
        let entries = vec![
            bundle_entry(
                "definition.json",
                serde_json::json!({
                    "package": "acme",
                    "version": "1.0.0",
                    "title": "Acme",
                    "description": "A bundle whose function reaches the network.",
                    "functions": {"main": "functions/main.zf.json"},
                    "nodes": [{
                        "kind": "n.x.acme.sync",
                        "title": "Sync",
                        "description": "Push a payload somewhere.",
                        "run": {"function": "main"},
                        "definition": {
                            "input_pins": ["in"],
                            "output_pins": ["out"],
                            "config_schema": {},
                            "input_schema": {"type": "object"},
                            "output_schema": {"type": "object"}
                        }
                    }]
                }),
            ),
            pipeline_entry(
                "functions/main.zf.json",
                &["n.trigger.function", "n.http.request"],
            ),
            pipeline_entry("pipelines/uses-it.zf.json", &["n.x.acme.sync"]),
        ];

        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &entries,
            Vec::new(),
            PackageReviewOptions {
                bundle_internal_paths: true,
                ..PackageReviewOptions::default()
            },
        );

        assert!(review.violations.is_empty(), "{:?}", review.violations);
        // The declared node reaches what its function composes, and the pipeline
        // that uses it is credited with the same.
        assert!(
            review
                .network_effects
                .contains(&"n.x.acme.sync".to_string()),
            "{:?}",
            review.network_effects
        );
        assert!(
            review
                .network_effects
                .contains(&"n.http.request".to_string())
        );
        assert!(
            !review
                .warnings
                .iter()
                .any(|warning| warning.contains("cannot say what these nodes do")),
            "the bundle accounts for its own node: {:?}",
            review.warnings
        );
    }

    /// The one refusal this change adds, and the whole argument for it: the
    /// kind is in the package's own bytes, the native catalog is a constant of
    /// this build, and a bundle that lands with a colliding kind makes the
    /// project's entire node registry fail to load -- not just its own node.
    #[test]
    fn a_bundle_claiming_a_kind_this_build_provides_is_refused() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[bundle_entry(
                "definition.json",
                serde_json::json!({
                    "package": "acme",
                    "version": "1.0.0",
                    "title": "Acme",
                    "description": "A bundle shadowing a native node.",
                    "functions": {"main": "functions/main.zf.json"},
                    "nodes": [{
                        "kind": "n.http.request",
                        "title": "HTTP Request",
                        "description": "Not the one you think.",
                        "run": {"function": "main"},
                        "definition": {
                            "input_pins": ["in"],
                            "output_pins": ["out"],
                            "config_schema": {},
                            "input_schema": {"type": "object"},
                            "output_schema": {"type": "object"}
                        }
                    }]
                }),
            )],
            Vec::new(),
            PackageReviewOptions {
                bundle_internal_paths: true,
                ..PackageReviewOptions::default()
            },
        );

        assert!(!review.is_installable());
        assert_eq!(review.risk_level, "blocked");
        assert_eq!(
            review.violations,
            vec![
                "definition.json: declares node kind 'n.http.request', which this build already \
                 provides"
                    .to_string()
            ]
        );
    }

    /// And it does not fire for a bundle that stayed in its own namespace,
    /// which is every legitimate one.
    #[test]
    fn a_bundle_in_its_own_namespace_is_not_refused() {
        let review = review_package_entries(
            &ResolvedProjectLayout::platform_default(),
            &[bundle_entry(
                "definition.json",
                serde_json::json!({
                    "package": "acme",
                    "version": "1.0.0",
                    "title": "Acme",
                    "description": "A bundle that owns the kind it declares.",
                    "functions": {"main": "functions/main.zf.json"},
                    "nodes": [{
                        "kind": "n.x.acme.request",
                        "title": "Acme Request",
                        "description": "Its own node.",
                        "run": {"function": "main"},
                        "definition": {
                            "input_pins": ["in"],
                            "output_pins": ["out"],
                            "config_schema": {},
                            "input_schema": {"type": "object"},
                            "output_schema": {"type": "object"}
                        }
                    }]
                }),
            )],
            Vec::new(),
            PackageReviewOptions {
                bundle_internal_paths: true,
                ..PackageReviewOptions::default()
            },
        );

        assert!(review.violations.is_empty(), "{:?}", review.violations);
    }

    /// A node bundle materializes into `data/hub/nodes/` under its own contract,
    /// not into anyone's repository, so the repository rule does not apply to
    /// it -- and a project that narrowed its own extensions cannot refuse a
    /// bundle that never touches its repository.
    #[test]
    fn a_node_bundle_is_judged_by_its_own_contract_not_the_repository_set() {
        let layout = ZebflowJsonLayout {
            allowed_extensions: Some(vec!["tsx".to_string()]),
            ..ZebflowJsonLayout::default()
        }
        .resolve();
        let entries = ["definition.json", "icon.svg", "wasm/core.wasm"]
            .iter()
            .map(|path| entry(path))
            .collect::<Vec<_>>();

        let review = review_package_entries(
            &layout,
            &entries,
            Vec::new(),
            PackageReviewOptions {
                bundle_internal_paths: true,
                ..PackageReviewOptions::default()
            },
        );

        assert!(review.violations.is_empty(), "{:?}", review.violations);
    }
}
