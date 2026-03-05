//! Minimal LLM client abstraction used by zebtune and framework nodes.

use std::sync::Arc;

use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmResponse {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmError {
    pub message: String,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LlmError {}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[LlmMessage]) -> Result<LlmResponse, LlmError>;
}

#[derive(Clone)]
struct OpenAiLikeClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

#[async_trait]
impl LlmClient for OpenAiLikeClient {
    async fn chat(&self, messages: &[LlmMessage]) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mapped = messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        LlmRole::System => "system",
                        LlmRole::User => "user",
                        LlmRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect::<Vec<_>>();

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let bearer = format!("Bearer {}", self.api_key);
        let auth = HeaderValue::from_str(&bearer).map_err(|err| LlmError {
            message: format!("invalid openai auth header: {}", err),
        })?;
        headers.insert(AUTHORIZATION, auth);

        let resp = self
            .http
            .post(url)
            .headers(headers)
            .json(&json!({
                "model": self.model,
                "messages": mapped,
            }))
            .send()
            .await
            .map_err(|err| LlmError {
                message: format!("openai request failed: {}", err),
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|err| LlmError {
            message: format!("openai parse failed: {}", err),
        })?;
        if !status.is_success() {
            return Err(LlmError {
                message: format!("openai error {}: {}", status, body),
            });
        }

        let content = body
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LlmError {
                message: "openai empty content".to_string(),
            })?
            .to_string();
        Ok(LlmResponse { content })
    }
}

#[derive(Clone)]
struct AnthropicClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn chat(&self, messages: &[LlmMessage]) -> Result<LlmResponse, LlmError> {
        let system = messages
            .iter()
            .filter(|m| m.role == LlmRole::System)
            .map(|m| m.content.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let mapped = messages
            .iter()
            .filter(|m| m.role != LlmRole::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        LlmRole::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": m.content,
                })
            })
            .collect::<Vec<_>>();

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        let key = HeaderValue::from_str(&self.api_key).map_err(|err| LlmError {
            message: format!("invalid anthropic api key header: {}", err),
        })?;
        headers.insert("x-api-key", key);

        let mut payload = json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": mapped,
        });
        if !system.is_empty() {
            payload["system"] = Value::String(system);
        }

        let resp = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|err| LlmError {
                message: format!("anthropic request failed: {}", err),
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|err| LlmError {
            message: format!("anthropic parse failed: {}", err),
        })?;
        if !status.is_success() {
            return Err(LlmError {
                message: format!("anthropic error {}: {}", status, body),
            });
        }

        let content = body
            .get("content")
            .and_then(Value::as_array)
            .and_then(|arr| {
                let mut out = Vec::new();
                for part in arr {
                    if part.get("type").and_then(Value::as_str) == Some("text")
                        && let Some(text) = part.get("text").and_then(Value::as_str)
                    {
                        out.push(text.trim().to_string());
                    }
                }
                if out.is_empty() {
                    None
                } else {
                    Some(out.join("\n"))
                }
            })
            .ok_or_else(|| LlmError {
                message: "anthropic empty content".to_string(),
            })?;

        Ok(LlmResponse { content })
    }
}

pub fn client_from_env() -> Option<Arc<dyn LlmClient>> {
    let provider = std::env::var("ZEBTUNE_LLM_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase();

    let http = reqwest::Client::builder().build().map_err(|_| ()).ok()?;

    if provider == "anthropic" {
        let api_key = std::env::var("ZEBTUNE_ANTHROPIC_API_KEY").ok()?;
        let model = std::env::var("ZEBTUNE_ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());
        return Some(Arc::new(AnthropicClient {
            http,
            api_key,
            model,
        }));
    }

    let api_key = std::env::var("ZEBTUNE_OPENAI_API_KEY").ok()?;
    let base_url = std::env::var("ZEBTUNE_OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("ZEBTUNE_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    Some(Arc::new(OpenAiLikeClient {
        http,
        base_url,
        api_key,
        model,
    }))
}
