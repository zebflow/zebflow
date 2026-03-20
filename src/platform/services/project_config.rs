//! Service for reading and writing `repo/zebflow.json` (Layer 2 project config).

use std::path::PathBuf;

use crate::platform::error::PlatformError;
use crate::platform::model::{
    ZebflowJson, ZebflowJsonAssistant, ZebflowJsonProject, ZebflowJsonRweLibraries,
    ZebflowJsonRweLibraryEntry, slug_segment,
};

/// Reads and writes `{data_root}/users/{owner}/{project}/repo/zebflow.json`.
pub struct ZebflowJsonService {
    users_root: PathBuf,
}

impl ZebflowJsonService {
    /// Creates service rooted at `{data_root}/users`.
    pub fn new(users_root: PathBuf) -> Self {
        Self { users_root }
    }

    fn json_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("repo")
            .join("zebflow.json")
    }

    /// Reads zebflow.json, returns default if missing.
    pub fn read_or_default(&self, owner: &str, project: &str) -> ZebflowJson {
        let path = self.json_path(owner, project);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return ZebflowJson::default();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    /// Writes zebflow.json atomically (best-effort).
    pub fn write(&self, owner: &str, project: &str, config: &ZebflowJson) -> Result<(), PlatformError> {
        let path = self.json_path(owner, project);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(config)
            .map_err(|e| PlatformError::new("ZEBFLOW_JSON_SERIALIZE", e.to_string()))?;
        std::fs::write(&path, serialized)
            .map_err(|e| PlatformError::new("ZEBFLOW_JSON_WRITE", e.to_string()))?;
        Ok(())
    }

    /// Reads zebflow.json, applies a mutation, and writes it back.
    pub fn update<F>(&self, owner: &str, project: &str, f: F) -> Result<ZebflowJson, PlatformError>
    where
        F: FnOnce(&mut ZebflowJson),
    {
        let mut cfg = self.read_or_default(owner, project);
        f(&mut cfg);
        self.write(owner, project, &cfg)?;
        Ok(cfg)
    }

    /// Writes the project title to zebflow.json, preserving other fields.
    pub fn set_project_title(
        &self,
        owner: &str,
        project: &str,
        title: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.project.title = title.to_string();
        })?;
        Ok(())
    }

    /// Gets the project title from zebflow.json, falling back to the project slug.
    pub fn get_project_title(&self, owner: &str, project: &str) -> String {
        let cfg = self.read_or_default(owner, project);
        if cfg.project.title.trim().is_empty() {
            project.replace('-', " ")
        } else {
            cfg.project.title.clone()
        }
    }

    /// Returns the assistant section of zebflow.json.
    pub fn get_assistant(&self, owner: &str, project: &str) -> ZebflowJsonAssistant {
        self.read_or_default(owner, project).assistant
    }

    /// Sets the assistant section of zebflow.json, preserving other fields.
    pub fn set_assistant(
        &self,
        owner: &str,
        project: &str,
        assistant: ZebflowJsonAssistant,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.assistant = assistant;
        })?;
        Ok(())
    }

    /// Returns the `rwe.libraries` map from zebflow.json.
    pub fn get_rwe_libraries(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonRweLibraries, PlatformError> {
        Ok(self.read_or_default(owner, project).rwe.libraries)
    }

    /// Adds or updates one enabled library entry in `rwe.libraries`.
    pub fn enable_rwe_library(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        version: &str,
        source: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.rwe.libraries.insert(
                name.to_string(),
                ZebflowJsonRweLibraryEntry {
                    version: version.to_string(),
                    source: source.to_string(),
                },
            );
        })?;
        Ok(())
    }

    /// Removes one library entry from `rwe.libraries`. No-op if not present.
    pub fn disable_rwe_library(
        &self,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.rwe.libraries.remove(name);
        })?;
        Ok(())
    }

    /// Initializes zebflow.json with defaults if it doesn't already exist.
    pub fn ensure_initialized(
        &self,
        owner: &str,
        project: &str,
        title: &str,
    ) -> Result<(), PlatformError> {
        let path = self.json_path(owner, project);
        if path.exists() {
            // Only update title if currently blank
            let mut cfg = self.read_or_default(owner, project);
            if cfg.project.title.trim().is_empty() && !title.trim().is_empty() {
                cfg.project.title = title.to_string();
                self.write(owner, project, &cfg)?;
            }
            return Ok(());
        }
        let cfg = ZebflowJson {
            project: ZebflowJsonProject {
                title: title.to_string(),
                description: String::new(),
            },
            ..Default::default()
        };
        self.write(owner, project, &cfg)
    }
}
