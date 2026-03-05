//! Project assistant configuration service.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectAssistantConfig, UpsertProjectAssistantConfigRequest, now_ts, slug_segment,
};

const DEFAULT_MAX_STEPS: u32 = 50;
const DEFAULT_MAX_REPLANS: u32 = 2;
const DEFAULT_ENABLED: bool = true;
const DEFAULT_CHAT_HISTORY_PAIRS: u32 = 10;
const MIN_MAX_STEPS: u32 = 1;
const MAX_MAX_STEPS: u32 = 1_000;
const MAX_MAX_REPLANS: u32 = 64;
const MIN_CHAT_HISTORY_PAIRS: u32 = 0;
const MAX_CHAT_HISTORY_PAIRS: u32 = 50;

/// Project-scoped assistant settings stored in metadata catalog.
pub struct AssistantConfigService {
    data: Arc<dyn DataAdapter>,
}

impl AssistantConfigService {
    /// Creates assistant config service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
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
        Ok(self
            .data
            .get_project_assistant_config(&owner, &project)?
            .unwrap_or_else(|| Self::default_config(&owner, &project)))
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

        let llm_high_credential_id = normalize_optional_id(req.llm_high_credential_id.as_deref());
        let llm_general_credential_id =
            normalize_optional_id(req.llm_general_credential_id.as_deref());
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let config = ProjectAssistantConfig {
            owner: owner.clone(),
            project: project.clone(),
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps: sanitize_max_steps(req.max_steps),
            max_replans: sanitize_max_replans(req.max_replans),
            enabled: req.enabled.unwrap_or(DEFAULT_ENABLED),
            chat_history_pairs: sanitize_chat_history_pairs(req.chat_history_pairs),
            updated_at: now_ts(),
        };
        self.data.put_project_assistant_config(&config)?;
        Ok(config)
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
            normalize_optional_id(config.llm_high_credential_id.as_deref());
        let llm_general_credential_id =
            normalize_optional_id(config.llm_general_credential_id.as_deref());
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let record = ProjectAssistantConfig {
            owner,
            project,
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps: sanitize_max_steps(Some(config.max_steps)),
            max_replans: sanitize_max_replans(Some(config.max_replans)),
            enabled: config.enabled,
            chat_history_pairs: sanitize_chat_history_pairs(Some(config.chat_history_pairs)),
            updated_at: if config.updated_at > 0 {
                config.updated_at
            } else {
                now_ts()
            },
        };
        self.data.put_project_assistant_config(&record)?;
        Ok(record)
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

fn normalize_optional_id(value: Option<&str>) -> Option<String> {
    value.map(slug_segment).filter(|v| !v.is_empty())
}

fn sanitize_max_steps(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DEFAULT_MAX_STEPS)
        .clamp(MIN_MAX_STEPS, MAX_MAX_STEPS)
}

fn sanitize_max_replans(value: Option<u32>) -> u32 {
    value.unwrap_or(DEFAULT_MAX_REPLANS).min(MAX_MAX_REPLANS)
}

fn sanitize_chat_history_pairs(value: Option<u32>) -> u32 {
    value
        .unwrap_or(DEFAULT_CHAT_HISTORY_PAIRS)
        .clamp(MIN_CHAT_HISTORY_PAIRS, MAX_CHAT_HISTORY_PAIRS)
}
