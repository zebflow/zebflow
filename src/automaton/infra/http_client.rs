//! OpenAI-compatible and Anthropic HTTP clients.
//! Both implement `LlmCall` — the single LLM interface used throughout automaton.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::llm_interface::{CallResult, LlmCall, Message, MessageRole, ToolCall, ToolDef, Usage};

// ── OpenAI-compatible client ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenAiApiSurface {
    Responses,
    ChatCompletions,
}

fn parse_response_surface(value: &str) -> Option<OpenAiApiSurface> {
    match value.trim().to_ascii_lowercase().as_str() {
        "responses" | "response" => Some(OpenAiApiSurface::Responses),
        "chat_completions" | "chat-completions" | "chat/completions" => {
            Some(OpenAiApiSurface::ChatCompletions)
        }
        _ => None,
    }
}

fn normalize_api_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/responses".to_string();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// OpenAI-family HTTP client.
///
/// `openai` credentials use the modern Responses API. `openrouter`
/// credentials keep the Chat Completions-compatible surface used by most
/// routing providers.
pub struct OpenAiHttpClient {
    base_url: String,
    response_path: String,
    api_key: String,
    pub model: String,
    client: reqwest::Client,
    surface: OpenAiApiSurface,
    store: bool,
}

impl OpenAiHttpClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self::new_responses(base_url, api_key, model)
    }

    pub fn new_responses(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            response_path: "/responses".to_string(),
            api_key,
            model,
            client: reqwest::Client::new(),
            surface: OpenAiApiSurface::Responses,
            store: false,
        }
    }

    pub fn new_chat_completions(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            response_path: "/chat/completions".to_string(),
            api_key,
            model,
            client: reqwest::Client::new(),
            surface: OpenAiApiSurface::ChatCompletions,
            store: false,
        }
    }

    /// Build from a credential secret JSON blob.
    /// Expected shape: `{ "api_key": "...", "base_url": "...", "model": "..." }`.
    pub fn from_secret(secret: &Value) -> Option<Self> {
        Self::from_secret_with_model(secret, None)
    }

    /// Like `from_secret` but allows overriding the model name.
    pub fn from_secret_with_model(secret: &Value, model_override: Option<&str>) -> Option<Self> {
        Self::from_secret_with_surface(
            secret,
            model_override,
            OpenAiApiSurface::Responses,
            "https://api.openai.com/v1",
        )
    }

    /// Build from a provider credential kind plus secret.
    pub fn from_provider_secret(
        provider_kind: &str,
        secret: &Value,
        model_override: Option<&str>,
    ) -> Option<Self> {
        let (default_surface, default_base_url) = match provider_kind {
            "openai" => (OpenAiApiSurface::Responses, "https://api.openai.com/v1"),
            "openrouter" => (
                OpenAiApiSurface::ChatCompletions,
                "https://openrouter.ai/api/v1",
            ),
            _ => return None,
        };
        Self::from_secret_with_surface(secret, model_override, default_surface, default_base_url)
    }

    fn from_secret_with_surface(
        secret: &Value,
        model_override: Option<&str>,
        surface: OpenAiApiSurface,
        default_base_url: &str,
    ) -> Option<Self> {
        let api_key = secret.get("api_key").and_then(|v| v.as_str())?;
        let surface = secret
            .get("response_surface")
            .and_then(|v| v.as_str())
            .and_then(parse_response_surface)
            .unwrap_or(surface);
        let base_url = secret
            .get("base_url")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(default_base_url)
            .to_string();
        let default_model = match surface {
            OpenAiApiSurface::Responses => "gpt-4o-mini",
            OpenAiApiSurface::ChatCompletions => "openai/gpt-4o-mini",
        };
        let default_response_path = match surface {
            OpenAiApiSurface::Responses => "/responses",
            OpenAiApiSurface::ChatCompletions => "/chat/completions",
        };
        let response_path = normalize_api_path(
            secret
                .get("response_path")
                .and_then(|v| v.as_str())
                .filter(|v| !v.trim().is_empty())
                .unwrap_or(default_response_path),
        );
        let model = model_override
            .filter(|m| !m.is_empty())
            .map(|m| m.to_string())
            .unwrap_or_else(|| {
                secret
                    .get("model")
                    .and_then(|v| v.as_str())
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or(default_model)
                    .to_string()
            });
        let store = secret
            .get("store")
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                Value::String(s) => Some(s.eq_ignore_ascii_case("true")),
                _ => None,
            })
            .unwrap_or(false);
        Some(Self {
            base_url,
            response_path,
            api_key: api_key.to_string(),
            model,
            client: reqwest::Client::new(),
            surface,
            store,
        })
    }
}

impl OpenAiHttpClient {
    fn response_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            normalize_api_path(&self.response_path)
        )
    }

    async fn call_chat_completions(&self, messages: Vec<Message>) -> Result<String, String> {
        let url = self.response_url();
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

    async fn call_responses(&self, messages: Vec<Message>) -> Result<String, String> {
        let url = self.response_url();
        let mut instructions = Vec::new();
        let input: Vec<Value> = messages
            .into_iter()
            .filter_map(|m| match m.role {
                MessageRole::System => {
                    if !m.content.trim().is_empty() {
                        instructions.push(m.content);
                    }
                    None
                }
                MessageRole::User => Some(json!({ "role": "user", "content": m.content })),
                MessageRole::Assistant => {
                    Some(json!({ "role": "assistant", "content": m.content }))
                }
            })
            .collect();

        let mut body = json!({ "model": self.model, "input": input, "store": self.store });
        if !instructions.is_empty() {
            body["instructions"] = Value::String(instructions.join("\n\n"));
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
        extract_responses_text(&val).ok_or_else(|| "empty LLM response".to_string())
    }

    async fn call_chat_completions_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<(CallResult, Usage), String> {
        let url = self.response_url();
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
        let usage = Usage::from_response(&val);
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
            return Ok((CallResult::ToolCalls(tool_calls), usage));
        }
        let text = choice
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        Ok((CallResult::Text(text), usage))
    }

    async fn call_responses_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<(CallResult, Usage), String> {
        let url = self.response_url();
        let tools_json: Vec<Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            })
            .collect();
        let (instructions, input) = responses_input_from_chat_messages(messages);
        let mut body = json!({ "model": self.model, "input": input, "store": self.store });
        if !instructions.is_empty() {
            body["instructions"] = Value::String(instructions);
        }
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

        let usage = Usage::from_response(&val);
        let tool_calls = val
            .get("output")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter(|item| {
                        item.get("type").and_then(Value::as_str) == Some("function_call")
                    })
                    .filter_map(|item| {
                        let id = item
                            .get("call_id")
                            .or_else(|| item.get("id"))?
                            .as_str()?
                            .to_string();
                        let name = item.get("name")?.as_str()?.to_string();
                        let arguments = item.get("arguments")?.as_str()?.to_string();
                        Some(ToolCall {
                            id,
                            name,
                            arguments,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if !tool_calls.is_empty() {
            return Ok((CallResult::ToolCalls(tool_calls), usage));
        }

        Ok((
            CallResult::Text(extract_responses_text(&val).unwrap_or_default()),
            usage,
        ))
    }
}

#[async_trait]
impl LlmCall for OpenAiHttpClient {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String> {
        match self.surface {
            OpenAiApiSurface::Responses => self.call_responses(messages).await,
            OpenAiApiSurface::ChatCompletions => self.call_chat_completions(messages).await,
        }
    }

    async fn call_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<(CallResult, Usage), String> {
        match self.surface {
            OpenAiApiSurface::Responses => self.call_responses_with_tools(messages, tools).await,
            OpenAiApiSurface::ChatCompletions => {
                self.call_chat_completions_with_tools(messages, tools).await
            }
        }
    }
}

fn responses_input_from_chat_messages(messages: Vec<Value>) -> (String, Vec<Value>) {
    let mut instructions = Vec::new();
    let mut input = Vec::new();

    for message in messages {
        match message.get("role").and_then(Value::as_str).unwrap_or("") {
            "system" => {
                if let Some(content) = message.get("content").and_then(Value::as_str) {
                    if !content.trim().is_empty() {
                        instructions.push(content.to_string());
                    }
                }
            }
            "user" | "assistant" => {
                if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
                    for call in tool_calls {
                        if let Some(function) = call.get("function") {
                            input.push(json!({
                                "type": "function_call",
                                "call_id": call.get("id").and_then(Value::as_str).unwrap_or(""),
                                "name": function.get("name").and_then(Value::as_str).unwrap_or(""),
                                "arguments": function.get("arguments").and_then(Value::as_str).unwrap_or("{}"),
                            }));
                        }
                    }
                    continue;
                }

                if let Some(content) = message.get("content").and_then(Value::as_str) {
                    input.push(json!({
                        "role": message.get("role").and_then(Value::as_str).unwrap_or("user"),
                        "content": content,
                    }));
                }
            }
            "tool" => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": message.get("tool_call_id").and_then(Value::as_str).unwrap_or(""),
                    "output": message.get("content").and_then(Value::as_str).unwrap_or(""),
                }));
            }
            _ => {}
        }
    }

    (instructions.join("\n\n"), input)
}

fn extract_responses_text(val: &Value) -> Option<String> {
    if let Some(text) = val.get("output_text").and_then(Value::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }

    let parts = val
        .get("output")
        .and_then(Value::as_array)?
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|content| {
            let kind = content.get("type").and_then(Value::as_str).unwrap_or("");
            if kind == "output_text" || kind == "text" {
                content
                    .get("text")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let text = parts.join("");
    if text.trim().is_empty() {
        None
    } else {
        Some(text.trim().to_string())
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

/// Build an `LlmCall` client from a provider credential kind and secret.
pub fn client_from_provider_secret_with_model(
    provider_kind: &str,
    secret: &Value,
    model_override: Option<&str>,
) -> Option<Arc<dyn LlmCall>> {
    OpenAiHttpClient::from_provider_secret(provider_kind, secret, model_override)
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
