//! Hub package policy scanner.
//!
//! This module owns the reusable package safety review used before publishing
//! or adding Hub packages. It deliberately returns facts instead of UI text
//! layout so Project Studio, CLI, and future APIs can render the same decision.

use std::collections::BTreeSet;
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
    /// Effects the installer reports and the user may accept.
    pub warnings: Vec<String>,
    /// Findings that make a package uninstallable.
    ///
    /// These are never overridable. A warning asks whether the user accepts an
    /// effect; a violation says the package will not be installed at all,
    /// whoever approves it. The contract validator already refuses malformed
    /// bundles; this refuses well-formed ones whose behaviour is unsafe.
    ///
    /// Declared now, enforced later: the fields the detectors need exist, but
    /// the detectors themselves are added once each is precise enough to be
    /// non-overridable without producing false positives.
    #[serde(default)]
    pub violations: Vec<String>,
    /// `low`, `medium`, `high`, or `blocked` when violations are present.
    pub risk_level: String,
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
        if entry.rel_path.ends_with(".zf.json") && entry.rel_path.starts_with("pipelines/") {
            if let Some(source) = entry.text() {
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
        if options.publish_mode && looks_like_private_publish_path(&entry.rel_path) {
            warnings.push(format!(
                "{} looks like private/runtime data",
                entry.rel_path
            ));
        }
    }

    warnings.sort();
    warnings.dedup();

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
        warnings,
        // No detectors yet. The tier exists so install can refuse on it and the
        // review dialog can distinguish "cannot be installed" from "are you
        // sure", before any detector is precise enough to be non-overridable.
        violations: Vec::new(),
        risk_level: PolicyRiskLevel::from_score(risk_score).as_str().to_string(),
    }
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
