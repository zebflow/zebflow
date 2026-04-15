//! OpenAI-compatible and Anthropic HTTP clients.
//! Both implement `LlmCall` — the single LLM interface used throughout automaton.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::llm_interface::{CallResult, LlmCall, Message, MessageRole, ToolCall, ToolDef};

// ── OpenAI-compatible client ──────────────────────────────────────────────────

/// OpenAI-compatible HTTP client (works with OpenAI, OpenRouter, MiniMax, etc.)
pub struct OpenAiHttpClient {
    base_url: String,
    api_key: String,
    pub model: String,
    client: reqwest::Client,
}

impl OpenAiHttpClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Build from a credential secret JSON blob.
    /// Expected shape: `{ "api_key": "...", "base_url": "...", "model": "..." }`.
    pub fn from_secret(secret: &Value) -> Option<Self> {
        Self::from_secret_with_model(secret, None)
    }

    /// Like `from_secret` but allows overriding the model name.
    pub fn from_secret_with_model(secret: &Value, model_override: Option<&str>) -> Option<Self> {
        let api_key = secret.get("api_key").and_then(|v| v.as_str())?;
        let base_url = secret
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1")
            .to_string();
        let model = model_override
            .filter(|m| !m.is_empty())
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                secret
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gpt-4o-mini")
                    .to_string()
            });
        Some(Self::new(base_url, api_key.to_string(), model))
    }
}

#[async_trait]
impl LlmCall for OpenAiHttpClient {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mapped: Vec<Value> = messages
            .into_iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                    },
                    "content": m.content,
                })
            })
            .collect();
        let body = json!({ "model": self.model, "messages": mapped });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("LLM error {}: {}", status, val));
        }
        val.get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| "empty LLM response".to_string())
    }

    async fn call_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<CallResult, String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let tools_json: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        let mut body = json!({ "model": self.model, "messages": messages });
        if !tools_json.is_empty() {
            body["tools"] = Value::Array(tools_json);
            body["tool_choice"] = json!("auto");
        }
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("LLM error {}: {}", status, val));
        }
        let choice = val
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .ok_or("empty choices")?;
        let finish_reason = choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("");
        if finish_reason == "tool_calls" {
            let tool_calls: Vec<ToolCall> = choice
                .get("message")
                .and_then(|m| m.get("tool_calls"))
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(|tc| {
                            let id = tc.get("id")?.as_str()?.to_string();
                            let name = tc.get("function")?.get("name")?.as_str()?.to_string();
                            let arguments =
                                tc.get("function")?.get("arguments")?.as_str()?.to_string();
                            Some(ToolCall {
                                id,
                                name,
                                arguments,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            return Ok(CallResult::ToolCalls(tool_calls));
        }
        let text = choice
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(CallResult::Text(text))
    }
}

// ── Anthropic client ──────────────────────────────────────────────────────────

/// Anthropic Messages API client.
/// `call_with_tools` falls back to text-only (uses default trait impl).
pub struct AnthropicClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmCall for AnthropicClient {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String> {
        let system = messages
            .iter()
            .filter(|m| m.role == MessageRole::System)
            .map(|m| m.content.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let mapped: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| {
                json!({
                    "role": match m.role {
                        MessageRole::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut payload = json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": mapped,
        });
        if !system.is_empty() {
            payload["system"] = Value::String(system);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("anthropic request failed: {}", e))?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .map_err(|e| format!("anthropic parse failed: {}", e))?;
        if !status.is_success() {
            return Err(format!("anthropic error {}: {}", status, body));
        }

        body.get("content")
            .and_then(Value::as_array)
            .and_then(|arr| {
                let parts: Vec<String> = arr
                    .iter()
                    .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|p| p.get("text").and_then(Value::as_str))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join("\n"))
                }
            })
            .ok_or_else(|| "anthropic empty content".to_string())
    }
    // call_with_tools falls back to text-only via default trait impl
}

// ── Factory functions ─────────────────────────────────────────────────────────

/// Build an `LlmCall` client from a credential secret JSON blob.
/// Shape: `{ "api_key": "...", "base_url": "...", "model": "..." }`.
pub fn client_from_secret(secret: &Value) -> Option<Arc<dyn LlmCall>> {
    client_from_secret_with_model(secret, None)
}

/// Like `client_from_secret` but allows overriding the model name.
pub fn client_from_secret_with_model(
    secret: &Value,
    model_override: Option<&str>,
) -> Option<Arc<dyn LlmCall>> {
    OpenAiHttpClient::from_secret_with_model(secret, model_override)
        .map(|c| Arc::new(c) as Arc<dyn LlmCall>)
}

/// Build an `LlmCall` client from environment variables.
/// ZEBTUNE_LLM_PROVIDER=openai (default) | anthropic
/// OpenAI: ZEBTUNE_OPENAI_API_KEY, ZEBTUNE_OPENAI_BASE_URL, ZEBTUNE_OPENAI_MODEL
/// Anthropic: ZEBTUNE_ANTHROPIC_API_KEY, ZEBTUNE_ANTHROPIC_MODEL
pub fn client_from_env() -> Option<Arc<dyn LlmCall>> {
    let provider = std::env::var("ZEBTUNE_LLM_PROVIDER")
        .unwrap_or_else(|_| "openai".to_string())
        .to_lowercase();

    if provider == "anthropic" {
        let api_key = std::env::var("ZEBTUNE_ANTHROPIC_API_KEY").ok()?;
        let model = std::env::var("ZEBTUNE_ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());
        return Some(Arc::new(AnthropicClient::new(api_key, model)));
    }

    let api_key = std::env::var("ZEBTUNE_OPENAI_API_KEY").ok()?;
    let base_url = std::env::var("ZEBTUNE_OPENAI_BASE_URL")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("ZEBTUNE_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    Some(Arc::new(OpenAiHttpClient::new(base_url, api_key, model)))
}
