//! Project assistant configuration service backed by `repo/zebflow.yaml`.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectAssistantConfig, UpsertProjectAssistantConfigRequest, ZebflowJsonAssistant, now_ts,
    slug_segment,
};
use crate::platform::services::project_config::ProjectConfigurationService;

const DEFAULT_MAX_STEPS: u32 = 50;
const DEFAULT_MAX_REPLANS: u32 = 2;
const DEFAULT_ENABLED: bool = true;
const DEFAULT_CHAT_HISTORY_PAIRS: u32 = 10;
const MIN_MAX_STEPS: u32 = 1;
const MAX_MAX_STEPS: u32 = 1_000;
const MAX_MAX_REPLANS: u32 = 64;
const MIN_CHAT_HISTORY_PAIRS: u32 = 0;
const MAX_CHAT_HISTORY_PAIRS: u32 = 50;

/// Project-scoped assistant settings stored in `repo/zebflow.yaml`.
pub struct AssistantConfigService {
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ProjectConfigurationService>,
}

impl AssistantConfigService {
    /// Creates assistant config service.
    pub fn new(data: Arc<dyn DataAdapter>, zebflow_cfg: Arc<ProjectConfigurationService>) -> Self {
        Self { data, zebflow_cfg }
    }

    /// Returns current config or default if missing.
    pub fn get_project_assistant_config(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectAssistantConfig, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_project_exists(&owner, &project)?;
        let assistant = self.zebflow_cfg.get_assistant(&owner, &project)?;
        Ok(self.assistant_to_config(&owner, &project, &assistant))
    }

    /// Upserts project assistant config.
    pub fn upsert_project_assistant_config(
        &self,
        owner: &str,
        project: &str,
        req: &UpsertProjectAssistantConfigRequest,
    ) -> Result<ProjectAssistantConfig, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_project_exists(&owner, &project)?;

        let llm_high_credential_id = normalize_optional_id(req.llm_high_credential_id.as_deref())?;
        let llm_general_credential_id =
            normalize_optional_id(req.llm_general_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let max_steps = bounded_value(
            "max_steps",
            req.max_steps.unwrap_or(DEFAULT_MAX_STEPS),
            MIN_MAX_STEPS,
            MAX_MAX_STEPS,
        )?;
        let max_replans = bounded_value(
            "max_replans",
            req.max_replans.unwrap_or(DEFAULT_MAX_REPLANS),
            0,
            MAX_MAX_REPLANS,
        )?;
        let chat_history_pairs = bounded_value(
            "chat_history_pairs",
            req.chat_history_pairs.unwrap_or(DEFAULT_CHAT_HISTORY_PAIRS),
            MIN_CHAT_HISTORY_PAIRS,
            MAX_CHAT_HISTORY_PAIRS,
        )?;

        let assistant = ZebflowJsonAssistant {
            high_model_credential: llm_high_credential_id.clone(),
            general_model_credential: llm_general_credential_id.clone(),
            max_steps: Some(max_steps),
            max_replans: Some(max_replans),
            enabled: Some(req.enabled.unwrap_or(DEFAULT_ENABLED)),
            chat_history_pairs: Some(chat_history_pairs),
        };
        self.zebflow_cfg
            .set_assistant(&owner, &project, assistant.clone())?;

        Ok(ProjectAssistantConfig {
            owner,
            project,
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps: assistant.max_steps.unwrap_or(DEFAULT_MAX_STEPS),
            max_replans: assistant.max_replans.unwrap_or(DEFAULT_MAX_REPLANS),
            enabled: assistant.enabled.unwrap_or(DEFAULT_ENABLED),
            chat_history_pairs: assistant
                .chat_history_pairs
                .unwrap_or(DEFAULT_CHAT_HISTORY_PAIRS),
            updated_at: now_ts(),
        })
    }

    /// Writes a fully materialized config payload.
    pub fn put_project_assistant_config(
        &self,
        config: &ProjectAssistantConfig,
    ) -> Result<ProjectAssistantConfig, PlatformError> {
        let owner = slug_segment(&config.owner);
        let project = slug_segment(&config.project);
        self.ensure_project_exists(&owner, &project)?;

        let llm_high_credential_id =
            normalize_optional_id(config.llm_high_credential_id.as_deref())?;
        let llm_general_credential_id =
            normalize_optional_id(config.llm_general_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let max_steps = bounded_value("max_steps", config.max_steps, MIN_MAX_STEPS, MAX_MAX_STEPS)?;
        let max_replans = bounded_value("max_replans", config.max_replans, 0, MAX_MAX_REPLANS)?;
        let chat_history_pairs = bounded_value(
            "chat_history_pairs",
            config.chat_history_pairs,
            MIN_CHAT_HISTORY_PAIRS,
            MAX_CHAT_HISTORY_PAIRS,
        )?;

        let assistant = ZebflowJsonAssistant {
            high_model_credential: llm_high_credential_id.clone(),
            general_model_credential: llm_general_credential_id.clone(),
            max_steps: Some(max_steps),
            max_replans: Some(max_replans),
            enabled: Some(config.enabled),
            chat_history_pairs: Some(chat_history_pairs),
        };
        self.zebflow_cfg
            .set_assistant(&owner, &project, assistant)?;

        Ok(ProjectAssistantConfig {
            owner,
            project,
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps,
            max_replans,
            enabled: config.enabled,
            chat_history_pairs,
            updated_at: if config.updated_at > 0 {
                config.updated_at
            } else {
                now_ts()
            },
        })
    }

    /// Project default assistant config.
    pub fn default_config(owner: &str, project: &str) -> ProjectAssistantConfig {
        ProjectAssistantConfig {
            owner: slug_segment(owner),
            project: slug_segment(project),
            llm_high_credential_id: None,
            llm_general_credential_id: None,
            max_steps: DEFAULT_MAX_STEPS,
            max_replans: DEFAULT_MAX_REPLANS,
            enabled: DEFAULT_ENABLED,
            chat_history_pairs: DEFAULT_CHAT_HISTORY_PAIRS,
            updated_at: now_ts(),
        }
    }

    fn assistant_to_config(
        &self,
        owner: &str,
        project: &str,
        a: &ZebflowJsonAssistant,
    ) -> ProjectAssistantConfig {
        ProjectAssistantConfig {
            owner: owner.to_string(),
            project: project.to_string(),
            llm_high_credential_id: a.high_model_credential.clone(),
            llm_general_credential_id: a.general_model_credential.clone(),
            max_steps: a.max_steps.unwrap_or(DEFAULT_MAX_STEPS),
            max_replans: a.max_replans.unwrap_or(DEFAULT_MAX_REPLANS),
            enabled: a.enabled.unwrap_or(DEFAULT_ENABLED),
            chat_history_pairs: a.chat_history_pairs.unwrap_or(DEFAULT_CHAT_HISTORY_PAIRS),
            updated_at: now_ts(),
        }
    }

    fn ensure_project_exists(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        if self.data.get_project(owner, project)?.is_some() {
            return Ok(());
        }
        Err(PlatformError::new(
            "PLATFORM_PROJECT_MISSING",
            format!("project '{owner}/{project}' not found"),
        ))
    }

    fn ensure_credential_exists(
        &self,
        owner: &str,
        project: &str,
        credential_id: Option<&str>,
    ) -> Result<(), PlatformError> {
        let Some(credential_id) = credential_id else {
            return Ok(());
        };
        if self
            .data
            .get_project_credential(owner, project, credential_id)?
            .is_some()
        {
            return Ok(());
        }
        Err(PlatformError::new(
            "PLATFORM_ASSISTANT_CONFIG_INVALID",
            format!("credential '{credential_id}' not found"),
        ))
    }
}

fn normalize_optional_id(value: Option<&str>) -> Result<Option<String>, PlatformError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(PlatformError::new(
            "PLATFORM_ASSISTANT_CONFIG_INVALID",
            "credential ids may contain only letters, numbers, dot, underscore, and hyphen",
        ));
    }
    Ok(Some(value.to_string()))
}

fn bounded_value(
    field: &str,
    value: u32,
    minimum: u32,
    maximum: u32,
) -> Result<u32, PlatformError> {
    if !(minimum..=maximum).contains(&value) {
        return Err(PlatformError::new(
            "PLATFORM_ASSISTANT_CONFIG_INVALID",
            format!("{field} must be between {minimum} and {maximum}"),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_values_are_validated_without_clamping() {
        assert_eq!(bounded_value("max_replans", 0, 0, 64).unwrap(), 0);
        assert_eq!(bounded_value("max_replans", 64, 0, 64).unwrap(), 64);
        assert!(bounded_value("max_replans", 65, 0, 64).is_err());
        assert!(bounded_value("max_steps", 0, 1, 1000).is_err());
    }

    #[test]
    fn credential_references_are_trimmed_but_not_rewritten() {
        assert_eq!(
            normalize_optional_id(Some(" ai_model.v1 ")).unwrap(),
            Some("ai_model.v1".to_string())
        );
        assert!(normalize_optional_id(Some("ai/model")).is_err());
    }
}
