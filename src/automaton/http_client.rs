//! Real OpenAI-compatible HTTP client for automaton tests/CLI.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::automaton::llm_interface::{CallResult, LlmCall, Message, MessageRole, ToolCall, ToolDef};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

/// OpenAI-compatible HTTP client.
pub struct OpenAiHttpClient {
    base_url: String,
    api_key: String,
    model: String,
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

    /// Build from env vars. Suffix = "" for primary, "_2" for secondary.
    pub fn from_env(suffix: &str) -> Option<Self> {
        let api_key = std::env::var(format!("ZEBTUNE_OPENAI_API_KEY{}", suffix)).ok()?;
        let base_url = std::env::var(format!("ZEBTUNE_OPENAI_BASE_URL{}", suffix))
            .unwrap_or_else(|_| "https://api.openai.com/v1".into());
        let model = std::env::var(format!("ZEBTUNE_OPENAI_MODEL{}", suffix))
            .unwrap_or_else(|_| "gpt-4o-mini".into());
        Some(Self::new(base_url, api_key, model))
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmCall for OpenAiHttpClient {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String> {
        let chat_messages: Vec<ChatMessage> = messages
            .into_iter()
            .map(|m| ChatMessage {
                role: match m.role {
                    MessageRole::System => "system".into(),
                    MessageRole::User => "user".into(),
                    MessageRole::Assistant => "assistant".into(),
                },
                content: m.content,
            })
            .collect();

        let req = ChatRequest {
            model: self.model.clone(),
            messages: chat_messages,
        };

        let resp = self
            .client
            .post(self.chat_url())
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        chat_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| "Empty response".into())
    }

    async fn call_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<CallResult, String> {
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

        let body = json!({
            "model": self.model,
            "messages": messages,
            "tools": tools_json,
            "tool_choice": "auto",
        });

        let resp = self
            .client
            .post(self.chat_url())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, body));
        }

        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse error: {}", e))?;

        let message = &data["choices"][0]["message"];

        // Check for tool calls first
        if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
            if !tool_calls.is_empty() {
                let calls: Vec<ToolCall> = tool_calls
                    .iter()
                    .filter_map(|tc| {
                        let id = tc.get("id")?.as_str()?.to_string();
                        let func = tc.get("function")?;
                        let name = func.get("name")?.as_str()?.to_string();
                        let arguments = func
                            .get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}")
                            .to_string();
                        Some(ToolCall { id, name, arguments })
                    })
                    .collect();
                if !calls.is_empty() {
                    return Ok(CallResult::ToolCalls(calls));
                }
            }
        }

        // Fall through to text content
        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Empty response".to_string())?
            .to_string();

        Ok(CallResult::Text(content))
    }
}
