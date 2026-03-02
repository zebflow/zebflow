//! Conversation history with auto-compression and token management.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::automaton::contract::Message;

/// Token counter trait (host provides implementation).
pub trait TokenCounter: Send + Sync {
    /// Count tokens in text.
    fn count(&self, text: &str) -> usize;
}

/// Simple token counter (rough estimate: 1 token ≈ 4 chars).
pub struct SimpleTokenCounter;

impl TokenCounter for SimpleTokenCounter {
    fn count(&self, text: &str) -> usize {
        (text.len() / 4).max(1)
    }
}

/// Conversation history with auto-compression.
pub struct ConversationHistory {
    messages: VecDeque<Message>,
    max_tokens: usize,
    current_tokens: usize,
}

impl ConversationHistory {
    /// Create new history with max token limit.
    pub fn new(max_tokens: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_tokens,
            current_tokens: 0,
        }
    }

    /// Add message and auto-compress if needed.
    pub fn add(&mut self, message: Message) {
        let tokens = Self::count_tokens(&message.content);
        self.current_tokens += tokens;
        self.messages.push_back(message);

        // Auto-compress if over limit
        if self.current_tokens > self.max_tokens {
            self.compress();
        }
    }

    /// Simple token counter (1 token ≈ 4 chars).
    fn count_tokens(text: &str) -> usize {
        (text.len() / 4).max(1)
    }

    /// Get all messages.
    pub fn messages(&self) -> &VecDeque<Message> {
        &self.messages
    }

    /// Get current token count.
    pub fn token_count(&self) -> usize {
        self.current_tokens
    }

    /// Compress old messages (keep recent, summarize old).
    fn compress(&mut self) {
        // Strategy: Keep last 10 messages, compress the rest
        if self.messages.len() <= 10 {
            return;
        }

        let keep_recent = 10;
        let to_compress = self.messages.len() - keep_recent;

        if to_compress == 0 {
            return;
        }

        // Extract old messages
        let mut old_messages = Vec::new();
        for _ in 0..to_compress {
            if let Some(msg) = self.messages.pop_front() {
                let tokens = Self::count_tokens(&msg.content);
                self.current_tokens = self.current_tokens.saturating_sub(tokens);
                old_messages.push(msg);
            }
        }

        // Create summary (simple for now - host can provide LLM-based summarizer)
        let summary = self.create_summary(&old_messages);
        let summary_tokens = Self::count_tokens(&summary);

        // Add summary as first message
        let summary_msg = Message {
            role: crate::automaton::contract::MessageRole::System,
            content: format!("[Previous conversation summary: {}]", summary),
            tool_call_id: None,
            tool_calls: None,
        };

        self.messages.push_front(summary_msg);
        self.current_tokens += summary_tokens;
    }

    /// Create simple summary of old messages.
    fn create_summary(&self, messages: &[Message]) -> String {
        format!("{} earlier messages (compressed)", messages.len())
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_tokens = 0;
    }
}

/// Token usage tracker for cost monitoring.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens (sent to LLM)
    pub input_tokens: usize,
    /// Output tokens (received from LLM)
    pub output_tokens: usize,
    /// Total API calls
    pub api_calls: usize,
}

impl TokenUsage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add usage from one API call.
    pub fn add(&mut self, input: usize, output: usize) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.api_calls += 1;
    }

    /// Total tokens.
    pub fn total_tokens(&self) -> usize {
        self.input_tokens + self.output_tokens
    }

    /// Estimate cost (rough: $0.15/1M input, $0.60/1M output for cheap models).
    pub fn estimate_cost_usd(&self) -> f64 {
        let input_cost = (self.input_tokens as f64) * 0.15 / 1_000_000.0;
        let output_cost = (self.output_tokens as f64) * 0.60 / 1_000_000.0;
        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automaton::contract::MessageRole;

    #[test]
    fn test_history_add() {
        let mut history = ConversationHistory::new(1000);
        history.add(Message {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_call_id: None,
            tool_calls: None,
        });
        assert_eq!(history.messages().len(), 1);
        assert!(history.token_count() > 0);
    }

    #[test]
    fn test_auto_compress() {
        let mut history = ConversationHistory::new(20); // Very low limit (20 tokens)

        // Add many messages to trigger compression (each ~2-3 tokens)
        for i in 0..20 {
            history.add(Message {
                role: MessageRole::User,
                content: format!("Message {}", i),
                tool_call_id: None,
                tool_calls: None,
            });
        }

        // Should have compressed (kept recent + summary)
        assert!(
            history.messages().len() <= 11,
            "Expected <= 11 messages, got {}",
            history.messages().len()
        );
    }

    #[test]
    fn test_token_usage() {
        let mut usage = TokenUsage::new();
        usage.add(1000, 500);
        usage.add(2000, 1000);

        assert_eq!(usage.input_tokens, 3000);
        assert_eq!(usage.output_tokens, 1500);
        assert_eq!(usage.total_tokens(), 4500);
        assert_eq!(usage.api_calls, 2);
        assert!(usage.estimate_cost_usd() > 0.0);
    }
}
