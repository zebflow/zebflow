//! LLM call interface (NO provider dependencies).

use async_trait::async_trait;

/// Abstract LLM interface. Host provides implementation.
#[async_trait]
pub trait LlmCall: Send + Sync {
    async fn call(&self, messages: Vec<Message>) -> Result<String, String>;
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
