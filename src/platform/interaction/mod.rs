//! Interaction engine: pattern-matches DSL output and returns a navigation URL.

pub mod patterns;

/// Matches executed DSL against known patterns to produce browser navigation.
pub struct InteractionEngine {
    owner: String,
    project: String,
}

impl InteractionEngine {
    pub fn new(owner: &str, project: &str) -> Self {
        Self {
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }

    /// Returns `Some(url)` if the DSL matched a pattern and execution was successful.
    pub fn match_dsl(&self, dsl: &str, ok: bool) -> Option<String> {
        if !ok {
            return None;
        }
        patterns::match_patterns(dsl.trim(), &self.owner, &self.project)
    }
}
