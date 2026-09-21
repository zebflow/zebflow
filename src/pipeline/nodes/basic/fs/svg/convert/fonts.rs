//! The fonts one conversion may name — a view over the platform's
//! [`FontService`](crate::platform::services::FontService): its database for
//! this project (bundled Inter plus `static/fonts/`) and the map from the
//! family an SVG names to the family the database registers. The node owns
//! no font bytes.

use std::collections::BTreeMap;
use std::sync::Arc;

use usvg::fontdb;

use super::ConvertError;
use crate::platform::services::{DEFAULT_FONT_FAMILY, FontService};

/// CSS generic families: alone in a stack they mean the bundled default.
const GENERIC_FAMILIES: &[&str] = &[
    "serif", "sans-serif", "monospace", "cursive", "fantasy", "system-ui", "ui-serif", "ui-sans-serif",
    "ui-monospace", "ui-rounded",
];

/// A font database plus the names an SVG may use for its families.
#[derive(Clone)]
pub struct FontSet {
    db: Arc<fontdb::Database>,
    /// declared name → the database's own family name
    names: BTreeMap<String, String>,
}

impl std::fmt::Debug for FontSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontSet").field("faces", &self.db.len()).field("names", &self.names).finish()
    }
}

impl FontSet {
    /// The bundled set alone — tests and the schema examples.
    pub fn bundled() -> Self {
        let db = FontService::bundled_db();
        let names = FontService::bundled_families()
            .into_iter()
            .flat_map(|f| {
                let actual = db
                    .faces()
                    .find(|face| face.families.first().is_some_and(|(n, _)| n == &f.family || n.starts_with(&f.family)))
                    .and_then(|face| face.families.first().map(|(n, _)| n.clone()))
                    .unwrap_or_else(|| f.family.clone());
                [(f.family.clone(), actual.clone()), (actual.clone(), actual)]
            })
            .collect();
        Self { db: Arc::new(db), names }
    }

    /// This project's set, from the service.
    pub fn for_project(fonts: &FontService, owner: &str, project: &str) -> Result<Self, ConvertError> {
        let db = fonts.fontdb_for(owner, project).map_err(|e| ConvertError::font(e.message))?;
        let mut names = BTreeMap::new();
        for family in fonts.families(owner, project).map_err(|e| ConvertError::font(e.message))? {
            // Ask the service for the registered name once per family; any
            // weight will do, the name is the same across faces.
            if let Ok(Some(face)) = fonts.resolve(owner, project, &family.family, family.weights.first().copied().unwrap_or(400)) {
                names.insert(family.family.clone(), face.family.clone());
                names.insert(face.family.clone(), face.family);
            }
        }
        Ok(Self { db, names })
    }

    pub fn db(&self) -> Arc<fontdb::Database> {
        self.db.clone()
    }

    /// The database's name for the bundled default family.
    pub fn default_family(&self) -> &str {
        self.names.get(DEFAULT_FONT_FAMILY).map(String::as_str).unwrap_or(DEFAULT_FONT_FAMILY)
    }

    /// The family name to write in the SVG for a declared family, or the
    /// refusal an SVG earns by naming one nobody has: it lists what does.
    pub fn resolve(&self, declared: &str) -> Result<&str, ConvertError> {
        self.names.get(declared.trim()).map(String::as_str).ok_or_else(|| self.refusal(declared))
    }

    /// A CSS `font-family` value — one name or a stack — resolved to the
    /// first family the project has. A stack of generic keywords only
    /// (`sans-serif`) means the bundled default; a stack naming nothing the
    /// project has is refused, with the list.
    pub fn resolve_stack(&self, stack: &str) -> Result<&str, ConvertError> {
        let candidates: Vec<&str> = stack
            .split(',')
            .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').trim())
            .filter(|s| !s.is_empty())
            .collect();
        for name in &candidates {
            if let Some(actual) = self.names.get(*name) {
                return Ok(actual);
            }
        }
        if !candidates.is_empty()
            && candidates.iter().all(|n| GENERIC_FAMILIES.contains(&n.to_ascii_lowercase().as_str()))
        {
            return Ok(self.default_family());
        }
        Err(self.refusal(stack))
    }

    fn refusal(&self, declared: &str) -> ConvertError {
        let mut known: Vec<&str> = self.declared();
        known.sort_unstable();
        ConvertError::font(format!(
            "font family '{declared}' is not available; this project has {} — add a .ttf/.otf under static/fonts/ to name another",
            known.join(", ")
        ))
    }

    /// The short names an SVG may use (one per family).
    pub fn declared(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for (declared, actual) in &self.names {
            if declared == actual && self.names.iter().any(|(d, a)| a == actual && d != actual) {
                continue;
            }
            out.push(declared.as_str());
        }
        out
    }

    /// The `usvg::Options` every parse in a conversion shares: this database
    /// and the bundled default as the fallback family.
    pub fn options<'a>(&self) -> usvg::Options<'a> {
        usvg::Options { fontdb: self.db(), font_family: self.default_family().to_string(), ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inter_is_bundled_under_its_declared_name_and_the_rest_is_refused_with_a_list() {
        let fonts = FontSet::bundled();
        assert_eq!(fonts.resolve("Inter").unwrap(), "Inter 24pt");
        assert_eq!(fonts.resolve("Inter 24pt").unwrap(), "Inter 24pt");
        assert_eq!(fonts.db().len(), 4);
        assert_eq!(fonts.declared(), vec!["Inter"]);
        let err = fonts.resolve("Comic Sans").unwrap_err();
        assert!(err.message.contains("not available") && err.message.contains("Inter"), "{}", err.message);
    }

    #[test]
    fn a_stack_takes_the_first_known_family_and_generic_keywords_mean_the_default() {
        let fonts = FontSet::bundled();
        assert_eq!(fonts.resolve_stack("'Fraunces', Inter, sans-serif").unwrap(), "Inter 24pt");
        assert_eq!(fonts.resolve_stack("sans-serif").unwrap(), "Inter 24pt");
        assert_eq!(fonts.resolve_stack("system-ui, sans-serif").unwrap(), "Inter 24pt");
        assert!(fonts.resolve_stack("Fraunces, Georgia").unwrap_err().message.contains("Inter"));
        assert!(fonts.resolve_stack("").is_err());
    }
}
