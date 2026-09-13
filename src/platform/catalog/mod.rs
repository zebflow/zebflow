//! Platform UI component catalog — shadcn-compatible Zeb React components
//! installable into user projects at `repo/pipelines/shared/ui/`.

use std::collections::HashMap;

use crate::platform::web::embedded::PLATFORM_SOURCE_LIBRARY_ASSETS;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::platform::model::ResolvedProjectLayout;
use crate::platform::policy::package::{
    PackagePolicyEntry, PackageReviewOptions, review_package_entries,
};

/// Where the catalog installs, relative to the project's source root.
const SHARED_UI_SUBDIR: &str = "shared/ui";

/// One entry in the UI component catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Component name slug, e.g. "button"
    pub name: String,
    /// Category group
    pub category: String,
    /// Short description
    pub description: String,
    /// Filename, e.g. "button.tsx"
    pub filename: String,
    /// True when the component already exists in the project
    #[serde(default)]
    pub installed: bool,
}

/// Result of an install operation.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CloneReport {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
}

/// Policy review for installing built-in UI components through Add+.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UiInstallReview {
    pub source: String,
    pub asset_kind: String,
    pub install_root: String,
    pub components: Vec<String>,
    pub files_added: Vec<String>,
    pub files_overwritten: Vec<String>,
    pub files_skipped: Vec<String>,
    pub nodes_used: Vec<String>,
    pub credentials_required: Vec<String>,
    pub external_urls: Vec<String>,
    pub database_effects: Vec<String>,
    pub filesystem_effects: Vec<String>,
    /// Node kinds that open an outbound connection, whether or not any URL is
    /// written down in a config.
    #[serde(default)]
    pub network_effects: Vec<String>,
    /// Node kinds that run code or a program the package supplied.
    #[serde(default)]
    pub code_execution: Vec<String>,
    pub public_endpoints: Vec<String>,
    pub schedules: Vec<String>,
    pub large_files: Vec<String>,
    pub seed_data: Vec<String>,
    pub warnings: Vec<String>,
    /// Findings that refuse the install outright; never overridable.
    #[serde(default)]
    pub violations: Vec<String>,
    pub risk_level: String,
}

/// Request to install UI components.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallUiRequest {
    pub names: Vec<String>,
    #[serde(default)]
    pub overwrite: bool,
}

// ── Component sources ────────────────────────────────────────────────────────

/// One component of `zeb/ui`, as the catalog lists it and as a clone copies it.
#[derive(Debug, Clone, Copy)]
pub struct UiSource {
    pub name: &'static str,
    pub source: &'static str,
    pub filename: &'static str,
    pub category: &'static str,
    pub description: &'static str,
}

/// The catalog is `zeb/ui`'s own source, `blessed/source-libraries/ui/<v>/src/`,
/// embedded by build.rs. A page imports a component from `zeb/ui/<name>`
/// without installing anything; the catalog exists for the project that wants
/// to *own* one — a clone copies these exact bytes into `shared/ui/`.
///
/// The description is the first sentence of the file's header comment, so
/// the catalog says what the file says.
pub fn ui_sources() -> Vec<UiSource> {
    let mut out: Vec<UiSource> = PLATFORM_SOURCE_LIBRARY_ASSETS
        .iter()
        .filter_map(|asset| {
            let rest = asset.path.strip_prefix("zeb/ui/")?;
            let (_version, file) = rest.split_once("/src/")?;
            if file.contains('/') || !file.ends_with(".tsx") {
                return None;
            }
            let name = file.trim_end_matches(".tsx");
            let source = std::str::from_utf8(asset.bytes).ok()?;
            Some(UiSource {
                name,
                source,
                filename: file,
                category: ui_category(name),
                description: header_sentence(source),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(b.name));
    out
}

fn ui_category(name: &str) -> &'static str {
    match name {
        "button" | "toggle" | "toggle-group" | "button-group" | "dropdown-menu" | "context-menu" => "actions",
        "input" | "textarea" | "label" | "field" | "checkbox" | "switch" | "radio-group" | "slider"
        | "input-otp" | "native-select" | "input-group" | "select" => "forms",
        "dialog" | "alert-dialog" | "sheet" | "drawer" | "popover" | "tooltip" | "hover-card" | "sonner" => "overlays",
        "tabs" | "accordion" | "collapsible" | "scroll-area" | "resizable" | "breadcrumb" | "pagination" => "navigation",
        "hooks" => "internal",
        _ => "display",
    }
}

/// The first sentence of the header comment: the line after the `/**` that
/// names the component, up to its first period.
fn header_sentence(source: &str) -> &'static str {
    let Some(start) = source.find("/**") else { return "" };
    let body = &source[start + 3..];
    let end = body.find("*/").unwrap_or(body.len());
    let text: String = body[..end]
        .lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let sentence = text.split(". ").next().unwrap_or("").trim_end_matches('.').to_string();
    // Leaked once per process; the catalog is read far more than built.
    Box::leak(sentence.into_boxed_str())
}

// ── CatalogService ─────────────────────────────────────────────────────────────

pub struct CatalogService;

impl CatalogService {
    /// Return all UI catalog entries (without presence info).
    pub fn list_ui() -> Vec<CatalogEntry> {
        ui_sources()
            .iter()
            .map(|UiSource { name, filename, category, description, .. }| CatalogEntry {
                name: name.to_string(),
                category: category.to_string(),
                description: description.to_string(),
                filename: filename.to_string(),
                installed: false,
            })
            .collect()
    }

    /// Return all UI catalog entries enriched with `installed` presence flag.
    pub fn list_ui_with_presence(shared_ui_dir: &PathBuf) -> Vec<CatalogEntry> {
        ui_sources()
            .iter()
            .map(|UiSource { name, filename, category, description, .. }| {
                let installed = shared_ui_dir.join(filename).exists();
                CatalogEntry {
                    name: name.to_string(),
                    category: category.to_string(),
                    description: description.to_string(),
                    filename: filename.to_string(),
                    installed,
                }
            })
            .collect()
    }

    /// Returns a map of `name → installed` for quick lookups.
    pub fn check_presence(shared_ui_dir: &PathBuf) -> HashMap<String, bool> {
        ui_sources()
            .iter()
            .map(|UiSource { name, filename, .. }| {
                let installed = shared_ui_dir.join(filename).exists();
                (name.to_string(), installed)
            })
            .collect()
    }

    /// Review installing UI components before writing to the project repo.
    pub fn review_ui(
        layout: &ResolvedProjectLayout,
        names: &[String],
        shared_ui_dir: &PathBuf,
        overwrite: bool,
    ) -> UiInstallReview {
        let install_root = layout.source_rel(SHARED_UI_SUBDIR);
        let sources = ui_sources();
        let source_map: HashMap<&str, (&str, &str)> = sources
            .iter()
            .map(|c| (c.name, (c.source, c.filename)))
            .collect();

        let mut components = Vec::new();
        let mut files_added = Vec::new();
        let mut files_overwritten = Vec::new();
        let mut files_skipped = Vec::new();
        let mut warnings = Vec::new();
        let mut policy_entries = Vec::new();

        for name in names {
            let Some((src, filename)) = source_map.get(name.as_str()) else {
                warnings.push(format!("unknown UI component '{name}'"));
                continue;
            };
            components.push(name.clone());
            let rel_path = format!("{install_root}/{filename}");
            let dest = shared_ui_dir.join(filename);
            if dest.exists() && overwrite {
                files_overwritten.push(rel_path.clone());
            } else if dest.exists() {
                files_skipped.push(rel_path.clone());
            } else {
                files_added.push(rel_path.clone());
            }
            // A built-in component is compiled in, so its bytes are always in
            // hand; they are still read through the review's own constructor,
            // which is the only thing that decides what a reviewer can see.
            policy_entries.push(PackagePolicyEntry::from_bytes(
                rel_path,
                "template",
                src.len(),
                src.as_bytes(),
            ));
        }

        if components.is_empty() {
            warnings.push("no valid UI components selected".to_string());
        }
        if !files_overwritten.is_empty() {
            warnings.push("install will overwrite existing shared UI files".to_string());
        }

        let mut policy = review_package_entries(
            layout,
            &policy_entries,
            warnings,
            PackageReviewOptions {
                publish_mode: true,
                ..PackageReviewOptions::default()
            },
        );
        if !files_overwritten.is_empty() && policy.risk_level == "low" {
            policy.risk_level = "medium".to_string();
        }

        UiInstallReview {
            source: "built_in".to_string(),
            asset_kind: "ui_components".to_string(),
            install_root,
            components,
            files_added,
            files_overwritten,
            files_skipped,
            nodes_used: policy.nodes_used,
            credentials_required: policy.credentials_required,
            external_urls: policy.external_urls,
            database_effects: policy.database_effects,
            filesystem_effects: policy.filesystem_effects,
            network_effects: policy.network_effects,
            code_execution: policy.code_execution,
            public_endpoints: policy.public_endpoints,
            schedules: policy.schedules,
            large_files: policy.large_files,
            seed_data: policy.seed_data,
            warnings: policy.warnings,
            violations: policy.violations,
            risk_level: policy.risk_level,
        }
    }

    /// Install the requested components with the shared package review as a
    /// gate: a violation refuses the install outright, exactly as it refuses
    /// every hub channel. The bytes reviewed here are the same bytes the
    /// seeded `zebflow.ui-*` template packages carry — the catalog stopped
    /// being an unreviewed channel when both facts became true.
    pub fn install_ui_reviewed(
        layout: &ResolvedProjectLayout,
        names: &[String],
        shared_ui_dir: &PathBuf,
        overwrite: bool,
    ) -> Result<CloneReport, String> {
        let review = Self::review_ui(layout, names, shared_ui_dir, overwrite);
        if !review.violations.is_empty() {
            return Err(format!(
                "components cannot be installed: {}",
                review.violations.join("; ")
            ));
        }
        Self::install_ui(names, shared_ui_dir, overwrite)
    }

    /// Install the requested components into `shared_ui_dir`.
    /// Returns a `CloneReport` describing what was installed vs skipped.
    pub fn install_ui(
        names: &[String],
        shared_ui_dir: &PathBuf,
        overwrite: bool,
    ) -> Result<CloneReport, String> {
        std::fs::create_dir_all(shared_ui_dir)
            .map_err(|e| format!("Failed to create shared/ui dir: {e}"))?;

        let sources = ui_sources();
        let source_map: HashMap<&str, (&str, &str)> = sources
            .iter()
            .map(|c| (c.name, (c.source, c.filename)))
            .collect();

        let mut report = CloneReport::default();

        for name in names {
            let Some((src, filename)) = source_map.get(name.as_str()) else {
                continue; // Unknown component — skip silently
            };
            let dest = shared_ui_dir.join(filename);
            if dest.exists() && !overwrite {
                report.skipped.push(name.clone());
                continue;
            }
            // The clone is the library's bytes plus one line of provenance.
            // Its `zeb/ui/*` imports still resolve to the library, so it
            // works unchanged; the page switches one import to `@/shared/ui/`.
            let cloned = format!(
                "// cloned from zeb/ui — {name}. Edit freely; `zeb/ui/*` imports still resolve to the library.\n{src}"
            );
            std::fs::write(&dest, cloned).map_err(|e| format!("Failed to write {filename}: {e}"))?;
            report.installed.push(name.clone());
        }

        Ok(report)
    }

    /// Get source content for a single component by name.
    pub fn get_source(name: &str) -> Option<&'static str> {
        ui_sources().into_iter().find(|c| c.name == name).map(|c| c.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_shared_ui_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("zebflow-catalog-{name}-{}", std::process::id()))
    }

    #[test]
    fn ui_install_review_reports_files_before_write() {
        let dir = temp_shared_ui_dir("review-files-before-write");
        let _ = std::fs::remove_dir_all(&dir);

        let review = CatalogService::review_ui(
            &ResolvedProjectLayout::platform_default(),
            &["button".to_string()],
            &dir,
            false,
        );

        assert_eq!(review.source, "built_in");
        assert_eq!(review.asset_kind, "ui_components");
        assert_eq!(review.install_root, "shared/ui");
        assert_eq!(review.components, vec!["button"]);
        assert_eq!(review.files_added, vec!["shared/ui/button.tsx"]);
        assert!(review.files_skipped.is_empty());
        assert!(review.files_overwritten.is_empty());
        assert_eq!(review.risk_level, "low");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ui_install_review_reports_skip_and_overwrite() {
        let dir = temp_shared_ui_dir("review-skip-overwrite");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("button.tsx"), "local edit").unwrap();

        let skipped = CatalogService::review_ui(
            &ResolvedProjectLayout::platform_default(),
            &["button".to_string()],
            &dir,
            false,
        );
        assert_eq!(skipped.files_skipped, vec!["shared/ui/button.tsx"]);
        assert!(skipped.files_overwritten.is_empty());

        let overwritten = CatalogService::review_ui(
            &ResolvedProjectLayout::platform_default(),
            &["button".to_string()],
            &dir,
            true,
        );
        assert!(overwritten.files_skipped.is_empty());
        assert_eq!(overwritten.files_overwritten, vec!["shared/ui/button.tsx"]);
        assert_eq!(overwritten.risk_level, "medium");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
