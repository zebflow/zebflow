//! Platform UI component catalog — shadcn-compatible Zeb React components
//! installable into user projects at `repo/pipelines/shared/ui/`.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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

/// Request to install UI components.
#[derive(Debug, Deserialize)]
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
                ($name, include_str!(concat!("ui/", $file)), $file, $cat, $desc),
            )*
        ]
    };
}

/// `(name, source, filename, category, description)`
static UI_SOURCES: &[(&str, &str, &str, &str, &str)] = ui_sources![
    // Primitives
    ("button",      "button.tsx",      "primitives", "Accessible button with variant and size props"),
    ("input",       "input.tsx",       "primitives", "Text input with consistent styling"),
    ("textarea",    "textarea.tsx",    "primitives", "Multi-line text input"),
    ("label",       "label.tsx",       "primitives", "Form label with peer-disabled support"),
    ("checkbox",    "checkbox.tsx",    "primitives", "Checkbox with onCheckedChange API"),
    ("radio-group", "radio-group.tsx", "primitives", "Radio group with single selection"),
    ("switch",      "switch.tsx",      "primitives", "Toggle switch with checked/onCheckedChange"),
    ("slider",      "slider.tsx",      "primitives", "Range slider with onValueChange"),
    // Display
    ("badge",      "badge.tsx",      "display", "Inline status badge with variants"),
    ("avatar",     "avatar.tsx",     "display", "Avatar with image and fallback"),
    ("progress",   "progress.tsx",   "display", "Progress bar 0–100"),
    ("skeleton",   "skeleton.tsx",   "display", "Loading skeleton placeholder"),
    ("separator",  "separator.tsx",  "display", "Horizontal or vertical divider"),
    ("kbd",        "kbd.tsx",        "display", "Keyboard shortcut display"),
    ("alert",      "alert.tsx",      "display", "Alert banner with title and description"),
    // Layout
    ("card",        "card.tsx",        "layout", "Card with header, content, and footer"),
    ("table",       "table.tsx",       "layout", "Styled HTML table with all sub-parts"),
    ("tabs",        "tabs.tsx",        "layout", "Tab panels with internal active state"),
    ("accordion",   "accordion.tsx",   "layout", "Collapsible accordion, single or multiple"),
    ("collapsible", "collapsible.tsx", "layout", "Simple open/close collapsible container"),
    ("scroll-area", "scroll-area.tsx", "layout", "Styled scrollable container"),
    // Navigation
    ("breadcrumb",    "breadcrumb.tsx",    "navigation", "Breadcrumb nav with all sub-parts"),
    ("pagination",    "pagination.tsx",    "navigation", "Page pagination with previous/next"),
    ("toggle",        "toggle.tsx",        "navigation", "Pressable toggle button"),
    ("toggle-group",  "toggle-group.tsx",  "navigation", "Toggle group with single or multiple selection"),
    // Overlay
    ("dialog",        "dialog.tsx",        "overlay", "Modal dialog with backdrop and close button"),
    ("alert-dialog",  "alert-dialog.tsx",  "overlay", "Confirmation dialog, no outside-click dismiss"),
    ("sheet",         "sheet.tsx",         "overlay", "Slide-in panel from any edge"),
    ("drawer",        "drawer.tsx",        "overlay", "Bottom drawer sheet"),
    ("popover",       "popover.tsx",       "overlay", "Anchored popover panel"),
    ("hover-card",    "hover-card.tsx",    "overlay", "Content card shown on hover"),
    ("tooltip",       "tooltip.tsx",       "overlay", "Tooltip shown on hover/focus"),
    ("dropdown-menu", "dropdown-menu.tsx", "overlay", "Dropdown menu with items, checkboxes, radios"),
    // Complex
    ("select",     "select.tsx",     "complex", "Custom select with item list"),
    ("sonner",     "sonner.tsx",     "complex", "Toast notifications with queue"),
    ("input-otp",  "input-otp.tsx",  "complex", "OTP input with auto-advance slots"),
    ("calendar",   "calendar.tsx",   "complex", "Month calendar with date selection"),
    ("data-table", "data-table.tsx", "complex", "Table with sorting, filtering, pagination"),
];

// ── CatalogService ─────────────────────────────────────────────────────────────

pub struct CatalogService;

impl CatalogService {
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
            std::fs::write(&dest, src)
                .map_err(|e| format!("Failed to write {filename}: {e}"))?;
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
