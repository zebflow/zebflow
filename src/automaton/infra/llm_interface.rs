//! LLM call interface (NO provider dependencies).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Abstract LLM interface. Host provides implementation.
#[async_trait]
pub trait LlmCall: Send + Sync {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String>;

    /// Native tool calling. Default falls back to text-only `call` (ignores tools).
    async fn call_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<CallResult, String> {
        // Fallback: strip to simple messages, call without tools
        let simple: Vec<Message> = messages
            .into_iter()
            .filter_map(|m| {
                let role = m.get("role")?.as_str()?;
                let content = m.get("content")?.as_str()?.to_string();
                let role = match role {
                    "system" => MessageRole::System,
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => return None,
                };
                Some(Message { role, content })
            })
            .collect();
        let _ = tools; // unused in fallback
        let text = self.call(simple).await?;
        Ok(CallResult::Text(text))
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Tool definition for native tool calling (OpenAI function calling schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON Schema object describing the function parameters.
    pub parameters: Value,
}

/// A single tool call requested by the model.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// JSON-encoded arguments string.
    pub arguments: String,
}

/// Result of `call_with_tools`.
#[derive(Debug, Clone)]
pub enum CallResult {
    /// Model produced a text response (no tool calls).
    Text(String),
    /// Model requested one or more tool calls.
    ToolCalls(Vec<ToolCall>),
}
