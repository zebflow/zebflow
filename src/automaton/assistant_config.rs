//! Project assistant LLM loader.
//!
//! Reads ProjectAssistantConfig + credentials from PlatformService,
//! builds dual LlmCall (high-level + general) for Zebtune.

use std::sync::Arc;

use crate::automaton::http_client::OpenAiHttpClient;
use crate::automaton::llm_interface::LlmCall;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;

/// Dual LLM pair loaded from project credentials.
pub struct ProjectAssistantLlm {
    /// High-level model: planning, decomposition, complex reasoning.
    pub high: Arc<dyn LlmCall>,
    /// General/cheap model: validation, simple tool calls.
    pub general: Arc<dyn LlmCall>,
    pub max_steps: u32,
    pub max_replans: u32,
    pub chat_history_pairs: u32,
}

/// Load dual LLM for a project from stored credentials.
///
/// Falls back to `general` for `high` when only one credential is configured.
pub fn load_project_assistant_llm(
    data: &dyn DataAdapter,
    owner: &str,
    project: &str,
) -> Result<ProjectAssistantLlm, PlatformError> {
    let config = data
        .get_project_assistant_config(owner, project)?
        .ok_or_else(|| {
            PlatformError::new(
                "ASSISTANT_NOT_CONFIGURED",
                format!("no assistant config for '{owner}/{project}'"),
            )
        })?;

    if !config.enabled {
        return Err(PlatformError::new(
            "ASSISTANT_DISABLED",
            format!("assistant is disabled for '{owner}/{project}'"),
        ));
    }

    let general = build_llm_from_credential(
        data,
        owner,
        project,
        config.llm_general_credential_id.as_deref(),
    )?
    .ok_or_else(|| {
        PlatformError::new(
            "ASSISTANT_NO_LLM",
            "assistant requires at least llm_general_credential_id to be set",
        )
    })?;

    // high falls back to general if not configured
    let high = build_llm_from_credential(
        data,
        owner,
        project,
        config.llm_high_credential_id.as_deref(),
    )?
    .unwrap_or_else(|| general.clone());

    Ok(ProjectAssistantLlm {
        high,
        general,
        max_steps: config.max_steps,
        max_replans: config.max_replans,
        chat_history_pairs: config.chat_history_pairs,
    })
}

fn build_llm_from_credential(
    data: &dyn DataAdapter,
    owner: &str,
    project: &str,
    credential_id: Option<&str>,
) -> Result<Option<Arc<dyn LlmCall>>, PlatformError> {
    let Some(credential_id) = credential_id else {
        return Ok(None);
    };

    let credential = data
        .get_project_credential(owner, project, credential_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "ASSISTANT_CREDENTIAL_MISSING",
                format!("credential '{credential_id}' not found"),
            )
        })?;

    let secret = &credential.secret;

    let api_key = secret
        .get("api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let base_url = secret
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.openai.com/v1")
        .to_string();

    let model = secret
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-4o-mini")
        .to_string();

    if api_key.is_empty() {
        return Err(PlatformError::new(
            "ASSISTANT_CREDENTIAL_INVALID",
            format!("credential '{credential_id}' missing api_key in secret"),
        ));
    }

    Ok(Some(Arc::new(OpenAiHttpClient::new(
        base_url, api_key, model,
    ))))
}
