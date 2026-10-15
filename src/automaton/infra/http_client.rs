//! The one HTTP client behind `LlmCall`: OpenAI-compatible, with two
//! surfaces — Responses (`/responses`) for the `openai` credential kind and
//! Chat Completions (`/chat/completions`) for `openrouter` — and the
//! credential-driven factories at the bottom. The only source of a key is
//! the credential secret handed in; there is no environment fallback.
//!
//! Anthropic is not a provider yet; its tool-use mapping is a separate decision.

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

const MAX_LLM_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// No total timeout of its own: a model may think for minutes, and the
/// engine's `--timeout` bounds the node, as in `http.response.fetch` and
/// `browser.page.run`.
fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn parse_llm_json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("invalid LLM JSON response: {e}"))
}

fn format_llm_status_error(status: reqwest::StatusCode, val: &Value) -> String {
    if let Some(msg) = val
        .get("error")
        .and_then(|e| e.get("message").or(Some(e)))
        .and_then(Value::as_str)
    {
        let snippet: String = msg.chars().take(512).collect();
        return format!("LLM error {status}: {snippet}");
    }
    format!("LLM error {status}")
}

/// The credential's `allowed_hosts`: a JSON list, or the form's text field
/// (hosts separated by commas, spaces or new lines).
fn allowed_hosts_from_secret(secret: &Value) -> Vec<String> {
    let items: Vec<String> = match secret.get("allowed_hosts") {
        Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(text)) => text.split([',', ' ', '\n', '\t']).map(str::to_string).collect(),
        _ => Vec::new(),
    };
    items.into_iter().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()).collect()
}

/// How a credential reaches a model on the project's own network, named in
/// every egress refusal so the fix is one read away.
const SELF_HOSTED_OPT_IN: &str = "a self-hosted model on a private address needs allow_private \
     set to true and its host listed in allowed_hosts on the LLM credential";

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
    allow_private: bool,
    allowed_hosts: Vec<String>,
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
            client: build_http_client(),
            surface: OpenAiApiSurface::Responses,
            store: false,
            allow_private: false,
            allowed_hosts: Vec::new(),
        }
    }

    pub fn new_chat_completions(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            response_path: "/chat/completions".to_string(),
            api_key,
            model,
            client: build_http_client(),
            surface: OpenAiApiSurface::ChatCompletions,
            store: false,
            allow_private: false,
            allowed_hosts: Vec::new(),
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
        let allow_private = secret
            .get("allow_private")
            .and_then(|v| match v {
                Value::Bool(b) => Some(*b),
                Value::String(s) => Some(s.trim().eq_ignore_ascii_case("true")),
                _ => None,
            })
            .unwrap_or(false);
        Some(Self {
            base_url,
            response_path,
            api_key: api_key.to_string(),
            model,
            client: build_http_client(),
            surface,
            store,
            allow_private,
            allowed_hosts: allowed_hosts_from_secret(secret),
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

    async fn post_json(&self, body: &Value) -> Result<Value, String> {
        let url = self.response_url();
        // As in `secure_request`: `allow_private` alone opens nothing; the
        // private host must also be named in `allowed_hosts`.
        if self.allow_private && self.allowed_hosts.is_empty() {
            return Err(format!(
                "FW_EGRESS_DENIED: llm.http allow_private names no host; {SELF_HOSTED_OPT_IN}"
            ));
        }
        let policy = crate::pipeline::security::OutboundHttpPolicy {
            allow_private: self.allow_private,
            allowed_hosts: self.allowed_hosts.clone(),
        };
        crate::pipeline::security::validate_outbound_http_url_with_policy(&url, "llm.http", &policy)
            .map_err(|e| {
                if e.code == "FW_EGRESS_DENIED" {
                    format!("{}: {}; {SELF_HOSTED_OPT_IN}", e.code, e.message)
                } else {
                    format!("{}: {}", e.code, e.message)
                }
            })?;
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let bytes = crate::pipeline::security::read_response_capped(resp, MAX_LLM_RESPONSE_BYTES, "FW_EGRESS")
            .await
            .map_err(|e| format!("LLM response: {}", e.message))?;
        // The status first: a 429 or a gateway's HTML 502 is an LLM error
        // with its status, never "invalid JSON".
        if !status.is_success() {
            let val = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
            return Err(format_llm_status_error(status, &val));
        }
        parse_llm_json(&bytes)
    }

    async fn call_chat_completions(&self, messages: Vec<Message>) -> Result<String, String> {
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
        let val = self.post_json(&body).await?;
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

        let val = self.post_json(&body).await?;
        extract_responses_text(&val).ok_or_else(|| "empty LLM response".to_string())
    }

    async fn call_chat_completions_with_tools(
        &self,
        messages: Vec<Value>,
        tools: &[ToolDef],
    ) -> Result<(CallResult, Usage), String> {
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
        let val = self.post_json(&body).await?;
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

        let val = self.post_json(&body).await?;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::{Json, Router, routing::post};
    use serde_json::json;
    use tokio::net::TcpListener;

    use super::{
        LlmCall, Message, MessageRole, OpenAiHttpClient, format_llm_status_error, parse_llm_json,
    };

    /// A local stand-in provider on loopback, opted in the way a self-hosted
    /// model is, answering every call with `answer`.
    async fn local_provider(
        answer: axum::response::Response,
    ) -> (OpenAiHttpClient, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let answer = Arc::new(std::sync::Mutex::new(Some(answer)));
        let app = Router::new().route(
            "/chat/completions",
            post(move || {
                let answer = answer.clone();
                async move { answer.lock().expect("lock").take().expect("one call") }
            }),
        );
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let secret = json!({
            "api_key": uuid::Uuid::new_v4().to_string(),
            "base_url": format!("http://{addr}"),
            "allow_private": true,
            "allowed_hosts": ["127.0.0.1"],
        });
        let client = OpenAiHttpClient::from_provider_secret("openrouter", &secret, None).expect("client");
        (client, server)
    }

    fn ask() -> Vec<Message> {
        vec![Message { role: MessageRole::User, content: "hi".to_string() }]
    }

    /// A 429 or a gateway's HTML 502 reads as "LLM error <status>", not as
    /// invalid JSON; a 200 still parses.
    #[tokio::test(flavor = "current_thread")]
    async fn llm_status_is_read_before_the_body_is_parsed_as_json() {
        use axum::response::IntoResponse;
        for (status, body) in [
            (axum::http::StatusCode::TOO_MANY_REQUESTS, "slow down"),
            (axum::http::StatusCode::BAD_GATEWAY, "<html><body>bad gateway</body></html>"),
        ] {
            let (client, server) = local_provider((status, body).into_response()).await;
            let err = client.call(ask()).await.expect_err("an error status is an error");
            server.abort();
            assert_eq!(err, format!("LLM error {status}"), "{err}");
        }
        let ok = Json(json!({ "choices": [{ "message": { "content": "hello" } }] })).into_response();
        let (client, server) = local_provider(ok).await;
        assert_eq!(client.call(ask()).await.expect("answers"), "hello");
        server.abort();
    }

    /// The self-hosted opt-in is `allow_private` plus an explicit host, as in
    /// `secure_request`: the flag alone reaches nothing, a listed host is
    /// reached (as the form's text or a JSON list), and every refusal names
    /// the opt-in.
    #[tokio::test(flavor = "current_thread")]
    async fn a_self_hosted_model_needs_allow_private_and_an_explicit_host() {
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let hits_route = hits.clone();
        let app = Router::new().route(
            "/chat/completions",
            post(move || {
                let hits_route = hits_route.clone();
                async move {
                    hits_route.fetch_add(1, Ordering::SeqCst);
                    Json(json!({ "choices": [{ "message": { "content": "local answer" } }] }))
                }
            }),
        );
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let client = |extra: serde_json::Value| {
            let mut secret = json!({
                "api_key": uuid::Uuid::new_v4().to_string(),
                "base_url": format!("http://{addr}"),
            });
            for (key, value) in extra.as_object().expect("object") {
                secret[key] = value.clone();
            }
            OpenAiHttpClient::from_provider_secret("openrouter", &secret, None).expect("client")
        };

        for refused in [
            json!({}),
            json!({ "allow_private": true }),
            json!({ "allow_private": "true" }),
            json!({ "allowed_hosts": ["127.0.0.1"] }),
            json!({ "allow_private": true, "allowed_hosts": "203.0.113.10" }),
        ] {
            let err = client(refused.clone()).call(ask()).await.expect_err("refused");
            assert!(err.starts_with("FW_EGRESS_DENIED"), "{refused}: {err}");
            assert!(err.contains("allow_private") && err.contains("allowed_hosts"), "{refused}: {err}");
        }
        assert_eq!(hits.load(Ordering::SeqCst), 0, "nothing refused was sent");

        let listed_as_text = client(json!({ "allow_private": "true", "allowed_hosts": "203.0.113.10, 127.0.0.1" }));
        assert_eq!(listed_as_text.call(ask()).await.expect("text list"), "local answer");
        let listed_as_json = client(json!({ "allow_private": true, "allowed_hosts": ["127.0.0.1"] }));
        assert_eq!(listed_as_json.call(ask()).await.expect("json list"), "local answer");
        server.abort();
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    /// No total timeout of the client's own: a model may think for minutes and
    /// the engine's `--timeout` bounds the node. reqwest exposes no getter for
    /// a client's timeout and a real wait would cost a minute, so the source
    /// outside the tests is read instead.
    #[test]
    fn the_client_sets_no_total_timeout_of_its_own() {
        let source = include_str!("http_client.rs");
        let code = &source[..source.find("#[cfg(test)]").expect("tests marker")];
        assert!(code.contains("fn build_http_client"), "the builder is still here");
        assert!(!code.contains(".timeout("), "the LLM client must not set a total timeout");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn openai_http_client_blocks_private_egress_and_caps_response_size() {
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let hits_route = hits.clone();
        let app = Router::new().route(
            "/responses",
            post(move || {
                let hits_route = hits_route.clone();
                async move {
                    hits_route.fetch_add(1, Ordering::SeqCst);
                    Json(json!({ "output_text": "internal secret" }))
                }
            }),
        );
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let client = OpenAiHttpClient::new(
            format!("http://{addr}"),
            uuid::Uuid::new_v4().to_string(),
            "gpt-4o-mini".to_string(),
        );
        let err = client
            .call(vec![Message {
                role: MessageRole::User,
                content: "hi".to_string(),
            }])
            .await
            .expect_err("private loopback base_url must be blocked before sending");
        server.abort();
        assert_eq!(hits.load(Ordering::SeqCst), 0, "loopback server should never be contacted");
        assert!(err.contains("FW_EGRESS_DENIED") || err.contains("blocked network address"), "err: {err}");

        // The size cap is the shared reader's (`read_response_capped`, tested
        // in `pipeline::security`); valid JSON still parses.
        let valid = parse_llm_json(br#"{"output_text":"hello"}"#).expect("valid");
        assert_eq!(valid["output_text"], "hello");
        assert_eq!(
            format_llm_status_error(reqwest::StatusCode::BAD_GATEWAY, &json!({ "upstream_html": "do-not-echo" })),
            "LLM error 502 Bad Gateway"
        );
    }
}
