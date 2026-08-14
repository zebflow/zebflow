//! Service for reading and writing `repo/zebflow.json` (Layer 2 project config).

use std::path::PathBuf;

use crate::infra::execution::placement::ProjectRuntimeProfile;
use crate::infra::execution::sync::ProjectBootstrapPlan;
use crate::infra::io::durable::{
    JsonContract, JsonContractField, JsonContractValue, read_optional_versioned_json,
    write_atomic_json,
};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ZebflowJson, ZebflowJsonAssistant, ZebflowJsonDistributionHub, ZebflowJsonMetadata,
    ZebflowJsonRweLibraries, ZebflowJsonRweLibraryEntry, slug_segment,
};

const ZEBFLOW_JSON_FIELDS: &[JsonContractField] = &[JsonContractField {
    name: "version",
    expected: JsonContractValue::String("1.0"),
}];
const ZEBFLOW_JSON_CONTRACT: JsonContract = JsonContract {
    name: "zebflow.json",
    fields: ZEBFLOW_JSON_FIELDS,
};

/// Returns true if `rel_path` matches a locked path or is inside a locked folder prefix.
pub fn is_template_path_locked(locked: &[String], rel_path: &str) -> bool {
    locked.iter().any(|p| {
        rel_path == p.as_str() || rel_path.starts_with(&format!("{}/", p.trim_end_matches('/')))
    })
}

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

    /// Reads `zebflow.json`, returning defaults only when the file is missing.
    ///
    /// Malformed and unsupported files are rejected so runtime behavior never
    /// silently changes because an authoritative project file is damaged.
    pub fn read_or_default(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJson, PlatformError> {
        let path = self.json_path(owner, project);
        read_optional_versioned_json(&path, ZEBFLOW_JSON_CONTRACT)
            .map(|config| config.unwrap_or_default())
            .map_err(|err| {
                PlatformError::new("ZEBFLOW_JSON_READ", format!("{} ({})", err, err.category()))
            })
    }

    /// Writes zebflow.json atomically (best-effort).
    pub fn write(
        &self,
        owner: &str,
        project: &str,
        config: &ZebflowJson,
    ) -> Result<(), PlatformError> {
        let path = self.json_path(owner, project);
        write_atomic_json(&path, config, ZEBFLOW_JSON_CONTRACT).map_err(|err| {
            PlatformError::new(
                "ZEBFLOW_JSON_WRITE",
                format!("{} ({})", err, err.category()),
            )
        })
    }

    /// Reads zebflow.json, applies a mutation, and writes it back.
    pub fn update<F>(&self, owner: &str, project: &str, f: F) -> Result<ZebflowJson, PlatformError>
    where
        F: FnOnce(&mut ZebflowJson),
    {
        let mut cfg = self.read_or_default(owner, project)?;
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
            cfg.metadata.title = title.to_string();
        })?;
        Ok(())
    }

    /// Gets the project title from zebflow.json, falling back to the project slug.
    pub fn get_project_title(&self, owner: &str, project: &str) -> Result<String, PlatformError> {
        let cfg = self.read_or_default(owner, project)?;
        if cfg.metadata.title.trim().is_empty() {
            Ok(project.replace('-', " "))
        } else {
            Ok(cfg.metadata.title.clone())
        }
    }

    /// Returns the hub distribution contract for one project.
    pub fn get_hub_distribution(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonDistributionHub, PlatformError> {
        Ok(self.read_or_default(owner, project)?.distribution.hub)
    }

    pub fn set_hub_distribution(
        &self,
        owner: &str,
        project: &str,
        hub: ZebflowJsonDistributionHub,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.distribution.hub = hub;
        })?;
        Ok(())
    }

    /// Returns the assistant section of zebflow.json.
    pub fn get_assistant(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonAssistant, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.assistant)
    }

    /// Returns the portable runtime profile section of `zebflow.json`.
    pub fn get_runtime_profile(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectRuntimeProfile, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.runtime)
    }

    /// Sets the portable runtime profile section of `zebflow.json`, preserving other fields.
    pub fn set_runtime_profile(
        &self,
        owner: &str,
        project: &str,
        runtime: ProjectRuntimeProfile,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.runtime = runtime;
        })?;
        Ok(())
    }

    /// Returns the bootstrap/activation plan section of `zebflow.json`.
    pub fn get_bootstrap(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectBootstrapPlan, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.bootstrap)
    }

    /// Sets the bootstrap/activation plan section of `zebflow.json`, preserving other fields.
    pub fn set_bootstrap(
        &self,
        owner: &str,
        project: &str,
        bootstrap: ProjectBootstrapPlan,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.bootstrap = bootstrap;
        })?;
        Ok(())
    }

    /// Sets the assistant section of zebflow.json, preserving other fields.
    pub fn set_assistant(
        &self,
        owner: &str,
        project: &str,
        assistant: ZebflowJsonAssistant,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            cfg.configs.assistant = assistant;
        })?;
        Ok(())
    }

    /// Returns the `rwe.libraries` map from zebflow.json.
    pub fn get_rwe_libraries(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ZebflowJsonRweLibraries, PlatformError> {
        Ok(self.read_or_default(owner, project)?.configs.rwe.libraries)
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
            cfg.configs.rwe.libraries.insert(
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
            cfg.configs.rwe.libraries.remove(name);
        })?;
        Ok(())
    }

    /// Returns whether `rel_path` is in the locked templates list.
    pub fn is_template_locked(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
    ) -> Result<bool, PlatformError> {
        let cfg = self.read_or_default(owner, project)?;
        Ok(is_template_path_locked(
            &cfg.configs.locks.templates,
            rel_path,
        ))
    }

    /// Adds or removes `rel_path` from the locked templates list.
    pub fn set_template_locked(
        &self,
        owner: &str,
        project: &str,
        rel_path: &str,
        locked: bool,
    ) -> Result<(), PlatformError> {
        self.update(owner, project, |cfg| {
            let templates = &mut cfg.configs.locks.templates;
            if locked {
                if !templates.iter().any(|p| p == rel_path) {
                    templates.push(rel_path.to_string());
                }
            } else {
                templates.retain(|p| p != rel_path);
            }
        })?;
        Ok(())
    }

    /// Returns all locked template paths from zebflow.json.
    pub fn get_locked_templates(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<String>, PlatformError> {
        Ok(self
            .read_or_default(owner, project)?
            .configs
            .locks
            .templates)
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
            let mut cfg = self.read_or_default(owner, project)?;
            if cfg.metadata.title.trim().is_empty() && !title.trim().is_empty() {
                cfg.metadata.title = title.to_string();
                self.write(owner, project, &cfg)?;
            }
            return Ok(());
        }
        let cfg = ZebflowJson {
            metadata: ZebflowJsonMetadata {
                title: title.to_string(),
                description: String::new(),
            },
            ..Default::default()
        };
        self.write(owner, project, &cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_defaults_but_malformed_and_future_configs_fail() {
        let root = tempfile::tempdir().unwrap();
        let service = ZebflowJsonService::new(root.path().join("users"));
        assert_eq!(
            service.read_or_default("owner", "project").unwrap().version,
            "1.0"
        );

        let path = service.json_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"not-json").unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap_err()
                .code,
            "ZEBFLOW_JSON_READ"
        );

        std::fs::write(&path, br#"{"version":"2.0"}"#).unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap_err()
                .code,
            "ZEBFLOW_JSON_READ"
        );
    }

    #[test]
    fn config_write_is_strict_and_roundtrips() {
        let root = tempfile::tempdir().unwrap();
        let service = ZebflowJsonService::new(root.path().join("users"));
        let mut config = ZebflowJson::default();
        config.metadata.title = "Stable".to_string();
        service.write("owner", "project", &config).unwrap();
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap()
                .metadata
                .title,
            "Stable"
        );

        config.version = "2.0".to_string();
        assert!(service.write("owner", "project", &config).is_err());
        assert_eq!(
            service
                .read_or_default("owner", "project")
                .unwrap()
                .metadata
                .title,
            "Stable"
        );
    }
}
