//! Fonts, for every renderer the platform has and a project's brand faces.
//!
//! Two sources, one database per project:
//!
//! - **Bundled** — Inter 400/500/600/800 (`runtime/fonts/inter/`, SIL OFL 1.1,
//!   `OFL.txt` beside the files), compiled into the binary so the same bytes
//!   shape the same glyphs on every server.
//! - **Project** — the files under the repository's `static/fonts/`: `.ttf`,
//!   `.otf` and `.ttc` are loaded into the database; `.woff2` is listed for
//!   the browser (a theme's `@font-face`) but not loaded, because the shaper
//!   reads raw font tables and a WOFF2 is a compressed container.
//!
//! Never a system font. A caller names a **family**, not a file: `resolve`
//! answers the face for a family and weight, `families` says what exists.
//! The per-project database is cached and rebuilt when `static/fonts/`
//! changes — the listing (names, sizes, mtimes) is the fingerprint, so a
//! write from any door (API, MCP, git) is seen without a hook.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use usvg::fontdb;

use crate::platform::adapters::file::FileAdapter;
use crate::platform::error::PlatformError;

/// The repository folder a project's fonts live in, under its source root.
pub const PROJECT_FONTS_DIR: &str = "static/fonts";
/// The family every document may name without a file.
pub const DEFAULT_FONT_FAMILY: &str = "Inter";
/// The bundled weights.
pub const BUNDLED_WEIGHTS: &[u16] = &[400, 500, 600, 800];
const BUNDLED: &[(&str, &[u8])] = &[
    ("Inter-Regular.ttf", include_bytes!("../../../runtime/fonts/inter/Inter-Regular.ttf")),
    ("Inter-Medium.ttf", include_bytes!("../../../runtime/fonts/inter/Inter-Medium.ttf")),
    ("Inter-SemiBold.ttf", include_bytes!("../../../runtime/fonts/inter/Inter-SemiBold.ttf")),
    ("Inter-ExtraBold.ttf", include_bytes!("../../../runtime/fonts/inter/Inter-ExtraBold.ttf")),
];
const LOADABLE: &[&str] = &["ttf", "otf", "ttc"];
const LISTED: &[&str] = &["ttf", "otf", "ttc", "woff2"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FontSource {
    Bundled,
    Project,
}

/// One family as a caller may name it.
#[derive(Debug, Clone, Serialize)]
pub struct FontFamily {
    /// The name to write — `Inter`, `Source Serif 4`.
    pub family: String,
    pub source: FontSource,
    /// Weights with a face of their own, ascending.
    pub weights: Vec<u16>,
    /// The files behind it (bare names under `static/fonts/`, or the bundled names).
    pub files: Vec<String>,
}

/// One face, chosen for a family and a weight.
#[derive(Debug, Clone)]
pub struct FontRef {
    /// The family name the database registers — what an SVG's `font-family`
    /// must say (Inter's files say "Inter 24pt").
    pub family: String,
    /// The weight of the face found, which may be the nearest, not the asked.
    pub weight: u16,
    pub source: FontSource,
    pub id: fontdb::ID,
}

struct ProjectFonts {
    fingerprint: String,
    db: Arc<fontdb::Database>,
    families: Vec<FontFamily>,
    /// declared name (and alias) → the database's own family name
    names: BTreeMap<String, String>,
    /// every file under `static/fonts/` the browser could use, bare names
    files: Vec<String>,
}

pub struct FontService {
    file: Arc<dyn FileAdapter>,
    cache: Mutex<HashMap<String, Arc<ProjectFonts>>>,
}

impl FontService {
    pub fn new(file: Arc<dyn FileAdapter>) -> Self {
        Self { file, cache: Mutex::new(HashMap::new()) }
    }

    /// The bundled faces alone — what every project has before it adds any.
    pub fn bundled_db() -> fontdb::Database {
        let mut db = fontdb::Database::new();
        for (_, bytes) in BUNDLED {
            db.load_font_data(bytes.to_vec());
        }
        db
    }

    pub fn bundled_families() -> Vec<FontFamily> {
        let db = Self::bundled_db();
        describe(&db, 0, FontSource::Bundled, &BUNDLED.iter().map(|(n, _)| n.to_string()).collect::<Vec<_>>()).0
    }

    /// Bundled + the project's `static/fonts/`, cached until that folder changes.
    pub fn fontdb_for(&self, owner: &str, project: &str) -> Result<Arc<fontdb::Database>, PlatformError> {
        Ok(self.project(owner, project)?.db.clone())
    }

    /// What a document may name, bundled first.
    pub fn families(&self, owner: &str, project: &str) -> Result<Vec<FontFamily>, PlatformError> {
        Ok(self.project(owner, project)?.families.clone())
    }

    /// The face for `family` at `weight` (nearest weight wins), or `None` when
    /// no such family exists. `family` may be the declared name (`Inter`) or
    /// the database's own (`Inter 24pt`).
    pub fn resolve(&self, owner: &str, project: &str, family: &str, weight: u16) -> Result<Option<FontRef>, PlatformError> {
        let fonts = self.project(owner, project)?;
        Ok(resolve_in(&fonts, family, weight))
    }

    /// The bare file names under `static/fonts/`, WOFF2 included — for a
    /// theme's `@font-face`.
    pub fn project_font_files(&self, owner: &str, project: &str) -> Result<Vec<String>, PlatformError> {
        Ok(self.project(owner, project)?.files.clone())
    }

    /// Forget a project's database; the next call rebuilds it. The
    /// fingerprint makes this unnecessary for a change under `static/fonts/`,
    /// but a project deletion or transfer may want it gone now.
    pub fn invalidate(&self, owner: &str, project: &str) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).remove(&format!("{owner}/{project}"));
    }

    fn project(&self, owner: &str, project: &str) -> Result<Arc<ProjectFonts>, PlatformError> {
        let layout = self.file.ensure_project_layout(owner, project)?;
        let dir = layout.repo_source_dir().join(PROJECT_FONTS_DIR);
        let mut entries: Vec<(String, u64, u64)> = Vec::new();
        if let Ok(read) = std::fs::read_dir(&dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
                if !LISTED.contains(&ext.as_str()) || !entry.path().is_file() {
                    continue;
                }
                let meta = entry.metadata().ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let mtime = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                entries.push((name, size, mtime));
            }
        }
        entries.sort();
        let fingerprint = entries.iter().map(|(n, s, m)| format!("{n}:{s}:{m}")).collect::<Vec<_>>().join("|");
        let key = format!("{owner}/{project}");
        if let Some(hit) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key)
            && hit.fingerprint == fingerprint
        {
            return Ok(hit.clone());
        }
        let mut db = Self::bundled_db();
        let bundled_count = db.len();
        let bundled_names: Vec<String> = BUNDLED.iter().map(|(n, _)| n.to_string()).collect();
        let (mut families, mut names) = describe(&db, 0, FontSource::Bundled, &bundled_names);
        let mut loaded_files = Vec::new();
        let mut files = Vec::new();
        for (name, _, _) in &entries {
            files.push(name.clone());
            let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
            if !LOADABLE.contains(&ext.as_str()) {
                continue;
            }
            let Ok(bytes) = std::fs::read(dir.join(name)) else { continue };
            let before = db.len();
            db.load_font_data(bytes);
            if db.len() > before {
                loaded_files.push(name.clone());
            }
        }
        let (project_families, project_names) = describe(&db, bundled_count, FontSource::Project, &loaded_files);
        families.extend(project_families);
        // A project face wins a name collision: the project chose it.
        names.extend(project_names);
        let fonts = Arc::new(ProjectFonts { fingerprint, db: Arc::new(db), families, names, files });
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(key, fonts.clone());
        Ok(fonts)
    }
}

/// The families among the faces from `from` onwards, and the names that
/// reach them: the database's own name, plus a short alias without an
/// optical-size suffix ("Inter 24pt" is also "Inter").
fn describe(db: &fontdb::Database, from: usize, source: FontSource, files: &[String]) -> (Vec<FontFamily>, BTreeMap<String, String>) {
    let mut by_family: BTreeMap<String, Vec<u16>> = BTreeMap::new();
    for face in db.faces().skip(from) {
        let Some((name, _)) = face.families.first() else { continue };
        by_family.entry(name.clone()).or_default().push(face.weight.0);
    }
    let mut names = BTreeMap::new();
    let mut families = Vec::new();
    for (actual, mut weights) in by_family {
        weights.sort_unstable();
        weights.dedup();
        names.insert(actual.clone(), actual.clone());
        let short = short_alias(&actual);
        if short != actual {
            names.entry(short.clone()).or_insert_with(|| actual.clone());
        }
        families.push(FontFamily { family: short, source, weights, files: files.to_vec() });
    }
    (families, names)
}

/// "Inter 24pt" → "Inter"; anything else unchanged.
fn short_alias(family: &str) -> String {
    match family.rsplit_once(' ') {
        Some((head, tail)) if tail.ends_with("pt") && tail.trim_end_matches("pt").chars().all(|c| c.is_ascii_digit()) && !head.is_empty() => head.to_string(),
        _ => family.to_string(),
    }
}

fn resolve_in(fonts: &ProjectFonts, family: &str, weight: u16) -> Option<FontRef> {
    let actual = fonts.names.get(family.trim())?;
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(actual)],
        weight: fontdb::Weight(weight),
        ..Default::default()
    };
    let id = fonts.db.query(&query)?;
    let face = fonts.db.face(id)?;
    let source = if fonts.families.iter().any(|f| f.source == FontSource::Project && (f.family == family.trim() || short_alias(actual) == f.family)) {
        FontSource::Project
    } else {
        FontSource::Bundled
    };
    Some(FontRef { family: actual.clone(), weight: face.weight.0, source, id })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;

    fn platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        config.default_password = "secret".to_string();
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    #[test]
    fn the_bundled_set_is_inter_at_four_weights_under_its_short_name() {
        let families = FontService::bundled_families();
        assert_eq!(families.len(), 1);
        assert_eq!(families[0].family, "Inter");
        assert_eq!(families[0].weights, vec![400, 500, 600, 800]);
        assert_eq!(families[0].source, FontSource::Bundled);
        assert_eq!(short_alias("Source Serif 4 36pt"), "Source Serif 4");
        assert_eq!(short_alias("Source Serif 4"), "Source Serif 4");
    }

    #[test]
    fn a_project_font_joins_the_bundled_set_and_the_cache_follows_the_folder() {
        let platform = platform();
        let fonts = &platform.fonts;
        let owner = "superadmin";
        let project = "default";
        let before = fonts.families(owner, project).unwrap();
        assert_eq!(before.iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), vec!["Inter"]);
        let inter = fonts.resolve(owner, project, "Inter", 800).unwrap().expect("Inter 800");
        assert_eq!(inter.weight, 800);
        assert_eq!(inter.family, "Inter 24pt");
        assert!(fonts.resolve(owner, project, "Fraunces", 400).unwrap().is_none());

        // Drop a copy of a bundled face into static/fonts/ under a new name:
        // it is a project font now, listed beside Inter, and a WOFF2 is
        // listed for the browser without being loaded.
        let layout = platform.file.ensure_project_layout(owner, project).unwrap();
        let dir = layout.repo_source_dir().join(PROJECT_FONTS_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Brand-Bold.ttf"), BUNDLED[3].1).unwrap();
        std::fs::write(dir.join("Brand-Bold.woff2"), b"wOF2 not a font").unwrap();
        std::fs::write(dir.join("notes.txt"), b"ignored").unwrap();
        let after = fonts.families(owner, project).unwrap();
        assert_eq!(after.len(), 2, "{after:?}");
        assert_eq!(after[1].source, FontSource::Project);
        assert_eq!(after[1].files, vec!["Brand-Bold.ttf"]);
        assert_eq!(fonts.project_font_files(owner, project).unwrap(), vec!["Brand-Bold.ttf", "Brand-Bold.woff2"]);
        assert_eq!(fonts.fontdb_for(owner, project).unwrap().len(), 5);
        // The nearest weight answers when the exact one is missing.
        let inter = fonts.resolve(owner, project, "Inter", 700).unwrap().expect("Inter near 700");
        assert!(inter.weight == 600 || inter.weight == 800);

        // Removing the file is seen without any hook.
        std::fs::remove_file(dir.join("Brand-Bold.ttf")).unwrap();
        assert_eq!(fonts.families(owner, project).unwrap().len(), 1);
    }
}
