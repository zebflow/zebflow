//! Platform UI component catalog — shadcn-compatible Zeb React components
//! installable into user projects at `repo/pipelines/shared/ui/`.

use std::collections::HashMap;
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

// ── Embedded component sources ────────────────────────────────────────────────

macro_rules! ui_sources {
    ( $( ($name:expr, $file:expr, $cat:expr, $desc:expr) ),* $(,)? ) => {
        &[
            $(
                (
                    $name,
                    include_str!(concat!("../../../blessed/templates/", $cat, "/", $file)),
                    $file,
                    $cat,
                    $desc,
                ),
            )*
        ]
    };
}

/// `(name, source, filename, category, description)`
static UI_SOURCES: &[(&str, &str, &str, &str, &str)] = ui_sources![
    // Primitives
    (
        "button",
        "button.tsx",
        "primitives",
        "Accessible button with variant and size props"
    ),
    (
        "input",
        "input.tsx",
        "primitives",
        "Text input with consistent styling"
    ),
    (
        "textarea",
        "textarea.tsx",
        "primitives",
        "Multi-line text input"
    ),
    (
        "label",
        "label.tsx",
        "primitives",
        "Form label with peer-disabled support"
    ),
    (
        "checkbox",
        "checkbox.tsx",
        "primitives",
        "Checkbox with onCheckedChange API"
    ),
    (
        "radio-group",
        "radio-group.tsx",
        "primitives",
        "Radio group with single selection"
    ),
    (
        "switch",
        "switch.tsx",
        "primitives",
        "Toggle switch with checked/onCheckedChange"
    ),
    (
        "slider",
        "slider.tsx",
        "primitives",
        "Range slider with onValueChange"
    ),
    // Display
    (
        "badge",
        "badge.tsx",
        "display",
        "Inline status badge with variants"
    ),
    (
        "avatar",
        "avatar.tsx",
        "display",
        "Avatar with image and fallback"
    ),
    ("progress", "progress.tsx", "display", "Progress bar 0–100"),
    (
        "skeleton",
        "skeleton.tsx",
        "display",
        "Loading skeleton placeholder"
    ),
    (
        "separator",
        "separator.tsx",
        "display",
        "Horizontal or vertical divider"
    ),
    ("kbd", "kbd.tsx", "display", "Keyboard shortcut display"),
    (
        "alert",
        "alert.tsx",
        "display",
        "Alert banner with title and description"
    ),
    // Layout
    (
        "card",
        "card.tsx",
        "layout",
        "Card with header, content, and footer"
    ),
    (
        "table",
        "table.tsx",
        "layout",
        "Styled HTML table with all sub-parts"
    ),
    (
        "tabs",
        "tabs.tsx",
        "layout",
        "Tab panels with internal active state"
    ),
    (
        "accordion",
        "accordion.tsx",
        "layout",
        "Collapsible accordion, single or multiple"
    ),
    (
        "collapsible",
        "collapsible.tsx",
        "layout",
        "Simple open/close collapsible container"
    ),
    (
        "scroll-area",
        "scroll-area.tsx",
        "layout",
        "Styled scrollable container"
    ),
    // Navigation
    (
        "breadcrumb",
        "breadcrumb.tsx",
        "navigation",
        "Breadcrumb nav with all sub-parts"
    ),
    (
        "pagination",
        "pagination.tsx",
        "navigation",
        "Page pagination with previous/next"
    ),
    (
        "toggle",
        "toggle.tsx",
        "navigation",
        "Pressable toggle button"
    ),
    (
        "toggle-group",
        "toggle-group.tsx",
        "navigation",
        "Toggle group with single or multiple selection"
    ),
    // Overlay
    (
        "dialog",
        "dialog.tsx",
        "overlay",
        "Modal dialog with backdrop and close button"
    ),
    (
        "alert-dialog",
        "alert-dialog.tsx",
        "overlay",
        "Confirmation dialog, no outside-click dismiss"
    ),
    (
        "sheet",
        "sheet.tsx",
        "overlay",
        "Slide-in panel from any edge"
    ),
    ("drawer", "drawer.tsx", "overlay", "Bottom drawer sheet"),
    (
        "popover",
        "popover.tsx",
        "overlay",
        "Anchored popover panel"
    ),
    (
        "hover-card",
        "hover-card.tsx",
        "overlay",
        "Content card shown on hover"
    ),
    (
        "tooltip",
        "tooltip.tsx",
        "overlay",
        "Tooltip shown on hover/focus"
    ),
    (
        "dropdown-menu",
        "dropdown-menu.tsx",
        "overlay",
        "Dropdown menu with items, checkboxes, radios"
    ),
    // Complex
    (
        "select",
        "select.tsx",
        "complex",
        "Custom select with item list"
    ),
    (
        "sonner",
        "sonner.tsx",
        "complex",
        "Toast notifications with queue"
    ),
    (
        "input-otp",
        "input-otp.tsx",
        "complex",
        "OTP input with auto-advance slots"
    ),
    (
        "calendar",
        "calendar.tsx",
        "complex",
        "Month calendar with date selection"
    ),
    (
        "data-table",
        "data-table.tsx",
        "complex",
        "Table with sorting, filtering, pagination"
    ),
];

/// The publish metadata beside each blessed template set, embedded so the
/// hub seeder can publish the sets without touching the source tree at run
/// time.
static TEMPLATE_SET_MANIFESTS: &[(&str, &str)] = &[
    (
        "primitives",
        include_str!("../../../blessed/templates/primitives/package.yaml"),
    ),
    (
        "display",
        include_str!("../../../blessed/templates/display/package.yaml"),
    ),
    (
        "layout",
        include_str!("../../../blessed/templates/layout/package.yaml"),
    ),
    (
        "navigation",
        include_str!("../../../blessed/templates/navigation/package.yaml"),
    ),
    (
        "overlay",
        include_str!("../../../blessed/templates/overlay/package.yaml"),
    ),
    (
        "complex",
        include_str!("../../../blessed/templates/complex/package.yaml"),
    ),
];

/// One blessed template set: the catalog's components grouped by category,
/// with the `package.yaml` the seeder publishes them under.
pub struct TemplateSet {
    /// Directory name under `blessed/templates/`, equal to the catalog
    /// category.
    pub set: &'static str,
    /// Raw `package.yaml` publish metadata.
    pub package_yaml: &'static str,
    /// `(filename, source)` for every component in the set.
    pub files: Vec<(&'static str, &'static str)>,
}

// ── CatalogService ─────────────────────────────────────────────────────────────

pub struct CatalogService;

impl CatalogService {
    /// The blessed template sets, grouped by catalog category.
    pub fn template_sets() -> Vec<TemplateSet> {
        TEMPLATE_SET_MANIFESTS
            .iter()
            .map(|(set, package_yaml)| TemplateSet {
                set,
                package_yaml,
                files: UI_SOURCES
                    .iter()
                    .filter(|(_, _, _, category, _)| category == set)
                    .map(|(_, source, filename, _, _)| (*filename, *source))
                    .collect(),
            })
            .collect()
    }

    /// Return all UI catalog entries (without presence info).
    pub fn list_ui() -> Vec<CatalogEntry> {
        UI_SOURCES
            .iter()
            .map(|(name, _, filename, category, description)| CatalogEntry {
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
        UI_SOURCES
            .iter()
            .map(|(name, _, filename, category, description)| {
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
        UI_SOURCES
            .iter()
            .map(|(name, _, filename, _, _)| {
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
        let source_map: HashMap<&str, (&str, &str)> = UI_SOURCES
            .iter()
            .map(|(name, src, filename, _, _)| (*name, (*src, *filename)))
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

        let source_map: HashMap<&str, (&str, &str)> = UI_SOURCES
            .iter()
            .map(|(name, src, filename, _, _)| (*name, (*src, *filename)))
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
            std::fs::write(&dest, src).map_err(|e| format!("Failed to write {filename}: {e}"))?;
            report.installed.push(name.clone());
        }

        Ok(report)
    }

    /// Get source content for a single component by name.
    pub fn get_source(name: &str) -> Option<&'static str> {
        UI_SOURCES
            .iter()
            .find(|(n, _, _, _, _)| *n == name)
            .map(|(_, src, _, _, _)| *src)
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
        assert_eq!(review.install_root, "pipelines/shared/ui");
        assert_eq!(review.components, vec!["button"]);
        assert_eq!(review.files_added, vec!["pipelines/shared/ui/button.tsx"]);
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
        assert_eq!(
            skipped.files_skipped,
            vec!["pipelines/shared/ui/button.tsx"]
        );
        assert!(skipped.files_overwritten.is_empty());

        let overwritten = CatalogService::review_ui(
            &ResolvedProjectLayout::platform_default(),
            &["button".to_string()],
            &dir,
            true,
        );
        assert!(overwritten.files_skipped.is_empty());
        assert_eq!(
            overwritten.files_overwritten,
            vec!["pipelines/shared/ui/button.tsx"]
        );
        assert_eq!(overwritten.risk_level, "medium");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
