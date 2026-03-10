//! Project assistant configuration service — reads/writes `repo/zebflow.json`.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectAssistantConfig, UpsertProjectAssistantConfigRequest, ZebflowJsonAssistant, now_ts,
    slug_segment,
};
use crate::platform::services::project_config::ZebflowJsonService;

const DEFAULT_MAX_STEPS: u32 = 50;
const DEFAULT_MAX_REPLANS: u32 = 2;
const DEFAULT_ENABLED: bool = true;
const DEFAULT_CHAT_HISTORY_PAIRS: u32 = 10;
const MIN_MAX_STEPS: u32 = 1;
const MAX_MAX_STEPS: u32 = 1_000;
const MAX_MAX_REPLANS: u32 = 64;
const MIN_CHAT_HISTORY_PAIRS: u32 = 0;
const MAX_CHAT_HISTORY_PAIRS: u32 = 50;

/// Project-scoped assistant settings stored in `repo/zebflow.json`.
pub struct AssistantConfigService {
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ZebflowJsonService>,
}

impl AssistantConfigService {
    /// Creates assistant config service.
    pub fn new(data: Arc<dyn DataAdapter>, zebflow_cfg: Arc<ZebflowJsonService>) -> Self {
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
        let assistant = self.zebflow_cfg.get_assistant(&owner, &project);
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

        let llm_high_credential_id = normalize_optional_id(req.llm_high_credential_id.as_deref());
        let llm_general_credential_id =
            normalize_optional_id(req.llm_general_credential_id.as_deref());
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let assistant = ZebflowJsonAssistant {
            high_model_credential: llm_high_credential_id.clone(),
            general_model_credential: llm_general_credential_id.clone(),
            max_steps: Some(sanitize_max_steps(req.max_steps)),
            max_replans: Some(sanitize_max_replans(req.max_replans)),
            enabled: Some(req.enabled.unwrap_or(DEFAULT_ENABLED)),
            chat_history_pairs: Some(sanitize_chat_history_pairs(req.chat_history_pairs)),
        };
        self.zebflow_cfg.set_assistant(&owner, &project, assistant.clone())?;

        Ok(ProjectAssistantConfig {
            owner,
            project,
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps: assistant.max_steps.unwrap_or(DEFAULT_MAX_STEPS),
            max_replans: assistant.max_replans.unwrap_or(DEFAULT_MAX_REPLANS),
            enabled: assistant.enabled.unwrap_or(DEFAULT_ENABLED),
            chat_history_pairs: assistant.chat_history_pairs.unwrap_or(DEFAULT_CHAT_HISTORY_PAIRS),
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
            normalize_optional_id(config.llm_high_credential_id.as_deref());
        let llm_general_credential_id =
            normalize_optional_id(config.llm_general_credential_id.as_deref());
        self.ensure_credential_exists(&owner, &project, llm_high_credential_id.as_deref())?;
        self.ensure_credential_exists(&owner, &project, llm_general_credential_id.as_deref())?;

        let max_steps = sanitize_max_steps(Some(config.max_steps));
        let max_replans = sanitize_max_replans(Some(config.max_replans));
        let chat_history_pairs = sanitize_chat_history_pairs(Some(config.chat_history_pairs));

        let assistant = ZebflowJsonAssistant {
            high_model_credential: llm_high_credential_id.clone(),
            general_model_credential: llm_general_credential_id.clone(),
            max_steps: Some(max_steps),
            max_replans: Some(max_replans),
            enabled: Some(config.enabled),
            chat_history_pairs: Some(chat_history_pairs),
        };
        self.zebflow_cfg.set_assistant(&owner, &project, assistant)?;

        Ok(ProjectAssistantConfig {
            owner,
            project,
            llm_high_credential_id,
            llm_general_credential_id,
            max_steps,
            max_replans,
            enabled: config.enabled,
            chat_history_pairs,
            updated_at: if config.updated_at > 0 { config.updated_at } else { now_ts() },
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
            max_steps: sanitize_max_steps(a.max_steps),
            max_replans: sanitize_max_replans(a.max_replans),
            enabled: a.enabled.unwrap_or(DEFAULT_ENABLED),
            chat_history_pairs: sanitize_chat_history_pairs(a.chat_history_pairs),
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
