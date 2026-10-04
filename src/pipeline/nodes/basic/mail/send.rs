//! `mail.message.send` — outgoing mail submission through a stored `smtp` credential.
//!
//! Submission only: this node hands one message to a relay the credential
//! names (port 587 STARTTLS by default). It is deliberately not a mail
//! server — no queueing, no retries, no DKIM. Deliverability belongs to the
//! relay.
//!
//! The message and the SMTP conversation are **mailbourne's**, the same
//! engine that runs the relay this may be pointing at. What stays here is
//! only what a mail engine cannot decide for its caller: which credential,
//! and what wraps the socket (see [`transport`](super::transport)).
//!
//! | Use | DSL |
//! |---|---|
//! | Activation email | `\| mail.message.send --credential relay --recipient "{{ input.email }}" --subject "Activate your account" --text "{{ input.body }}"` |
//! | Fixed recipients | `\| mail.message.send --credential relay --recipient ops@example.com --recipient oncall@example.com --subject "Backup done" --text "ok"` |
//!
//! `--recipient`, `--subject`, `--text`, `--html`, `--sender` and
//! `--reply-to` each take a literal or `{{ expr }}` — the one resolution
//! mechanism (`docs/contracts/kinds/node-io`). The engine resolves before
//! this node runs, so the config that arrives here is final.
//!
//! The answer, `message`, carries what was sent and to whom — never the
//! credential.

use std::sync::Arc;

use async_trait::async_trait;
use mailbourne::send::conversation::{Credentials, Outcome, Step};
use mailbourne::shared::compose;
use mailbourne::shared::core::{EmailAddress, Envelope};
use mailbourne::shared::mime::Attachment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};

pub const NODE_KIND: &str = "mail.message.send";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// The most recipients one message takes.
pub const MAX_RECIPIENTS: u32 = 50;
/// The most files one message attaches.
pub const MAX_FILES: u32 = 20;

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Credential],
        title: "Send Mail".to_string(),
        description: "Sends one email through a stored smtp credential's relay. It goes to every `--recipient` (repeat, up to 50), with \
            `--subject` and `--text` and/or `--html`; `--file` attaches a stored file (repeat; it arrives under the FileRef's filename, \
            or the key's last segment) and `--inline id=FILE` places a picture the HTML draws as `<img src=\"cid:id\">`. Every value \
            takes a literal or {{ expr }}. Adds `message: { sent, id, recipient, subject, attached }` and keeps the rest of the payload. \
            Submission only — deliverability (SPF, DKIM, reputation) is the relay's job."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `message` is added." }),
        output_schema: json!({ "type": "object", "properties": { "message": { "type": "object", "properties": {
            "sent": { "type": "boolean" },
            "id": { "type": "string", "description": "The Message-ID header it was sent with." },
            "recipient": { "type": "array", "items": { "type": "string" } },
            "subject": { "type": "string" },
            "attached": { "type": "array", "items": { "type": "string" }, "description": "Attachment filenames." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--credential", "credential_id", "The smtp credential naming the relay.", "text") },
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                required: true,
                max_repeat: Some(MAX_RECIPIENTS),
                ..flag("--recipient", "recipient", "An address to send to — repeat for several, or one {{ [list] }}.", "text")
            },
            DslFlag { required: true, ..flag("--subject", "subject", "The subject line.", "text") },
            flag("--text", "text", "The plain-text body.", "text"),
            flag("--html", "html", "The HTML body. Given both, the mail is multipart/alternative.", "text"),
            flag("--sender", "sender", "The From address, instead of the credential's.", "text"),
            flag("--reply-to", "reply_to", "Where replies go, when that is not the sender.", "text"),
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                max_repeat: Some(MAX_FILES),
                ..flag("--file", "file", "A file to attach: a FileRef or a store key, repeated. It arrives under the FileRef's filename, or the key's last segment.", "file")
            },
            DslFlag {
                kind: DslFlagKind::KeyValuePairs,
                ..flag("--inline", "inline", "A picture the HTML draws inline: id=FILE (FileRef or store key), placed with <img src=\"cid:id\">. Repeat for several; sent as a plain attachment when there is no --html.", "file")
            },
            crate::pipeline::nodes::shared::project_store::store_flag(),
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "SMTP Credential".to_string(), field_type: NodeFieldType::Text, help: Some("Credential of kind smtp naming the relay.".to_string()), ..Default::default() },
                NodeFieldDef { name: "recipient".to_string(), label: "Recipients".to_string(), field_type: NodeFieldType::Text, help: Some("An address, or {{ expr }} giving one or a list.".to_string()), ..Default::default() },
                NodeFieldDef { name: "subject".to_string(), label: "Subject".to_string(), field_type: NodeFieldType::Text, help: Some("Literal or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "text".to_string(), label: "Text Body".to_string(), field_type: NodeFieldType::Textarea, help: Some("Plain-text body — literal or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "html_section".to_string(), label: "HTML version (optional)".to_string(), field_type: NodeFieldType::Section, help: Some("An email carries the text above and, optionally, an HTML rendering of the same words. A reader's client shows one or the other — never both — so this is an addition, not a choice.".to_string()), ..Default::default() },
                NodeFieldDef { name: "html".to_string(), label: "HTML Body".to_string(), field_type: NodeFieldType::CodeEditor, language: Some("html".to_string()), rows: Some(12), help: Some("Leave empty to send plain text only. Email clients are not browsers: use tables and inline styles, and expect no JavaScript, no flexbox and no external stylesheet.".to_string()), ..Default::default() },
                NodeFieldDef { name: "file".to_string(), label: "Attachments".to_string(), field_type: NodeFieldType::Text, help: Some("A FileRef or store key, or {{ expr }} giving a list of them. Each arrives under its own filename.".to_string()), ..Default::default() },
                NodeFieldDef { name: "inline".to_string(), label: "Inline pictures".to_string(), field_type: NodeFieldType::KeyValuePairs, help: Some("Each row: a short id you choose, and the store key or FileRef it comes from. Write <img src=\"cid:id\"> in the HTML body to place it — a logo, say. Sent as a plain attachment when there is no HTML body.".to_string()), ..Default::default() },
                NodeFieldDef { name: "sender".to_string(), label: "Sender".to_string(), field_type: NodeFieldType::Text, help: Some("Overrides the credential's From address.".to_string()), ..Default::default() },
                NodeFieldDef { name: "reply_to".to_string(), label: "Reply-To".to_string(), field_type: NodeFieldType::Text, help: Some("Where replies should go when that is not the sender — literal or {{ expr }}.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("recipient".to_string()), LayoutItem::Field("subject".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("sender".to_string()), LayoutItem::Field("reply_to".to_string())] },
            LayoutItem::Field("text".to_string()),
            LayoutItem::Field("html_section".to_string()),
            LayoutItem::Field("html".to_string()),
            LayoutItem::Field("file".to_string()),
            LayoutItem::Field("inline".to_string()),
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Confirmation after a form", r#"mail.message.send --credential smtp_main --recipient "{{ $trigger.body.email }}" --subject "We got your message" --text "Thanks {{ $trigger.body.name }}, we will reply within a day.""#)
                .output(serde_json::json!({ "message": { "sent": true, "id": "<1759.mb@mail.example.com>", "recipient": ["a@example.com"], "subject": "We got your message", "attached": [] } }))
                .note("Adds `message` to the payload and keeps the rest. The credential is created by the owner in Studio → Credentials (kind smtp)."),
            crate::pipeline::model::NodeExample::dsl("A certificate as an attachment", r#"mail.message.send --credential smtp_main --recipient "{{ input.query.rows[0].email }}" --subject "Your certificate" --text "Attached." --file "{{ input.document }}""#),
        ],
        ..Default::default()
    }
}

/// A scalar flag with its 0.11 metadata.
fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub credential_id: String,
    /// Addresses: repeated, or one `{{ [list] }}` (a list inside the list
    /// is flattened).
    #[serde(default)]
    pub recipient: Value,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub html: Option<String>,
    /// The From address, instead of the credential's.
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Files to attach: FileRefs or store keys, repeated or one list.
    #[serde(default)]
    pub file: Value,
    /// Pictures the HTML body draws inline: your own short id → store key
    /// or FileRef. The HTML references one as `<img src="cid:ID">`. With no
    /// HTML body nothing could reference it, so it is sent as an ordinary
    /// attachment instead.
    #[serde(default)]
    pub inline: std::collections::BTreeMap<String, Value>,
    /// The store a bare key is read from; a FileRef names its own.
    #[serde(default)]
    pub store: Option<String>,
}

/// A list flag's values, a list inside the list flattened and empty strings
/// dropped: `--recipient a --recipient b` and `--recipient "{{ [a, b] }}"`
/// are the same two.
fn listed(value: &Value) -> Vec<Value> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().flat_map(listed).collect(),
        Value::String(s) if s.trim().is_empty() => Vec::new(),
        other => vec![other.clone()],
    }
}

/// An address string onto mailbourne's, keeping any display name out of it.
///
/// A credential's `from` is conventionally `Zebflow <mail@example.com>`,
fn to_mailbourne(raw: &str, what: &str) -> Result<EmailAddress, PipelineError> {
    let inner = match (raw.rfind('<'), raw.rfind('>')) {
        (Some(open), Some(close)) if close > open => &raw[open + 1..close],
        _ => raw,
    };
    EmailAddress::parse(inner.trim()).map_err(|e| {
        PipelineError::new("FW_NODE_MAIL_MESSAGE_SEND_ADDRESS", format!("{what} address '{raw}' is not valid: {e:?}"))
    })
}

/// Where in the SMTP conversation something happened, in words.
fn step_name(step: Step) -> &'static str {
    match step {
        Step::Greeting => "the greeting",
        Step::Ehlo => "the introduction",
        Step::Auth => "the password",
        Step::StartTls => "going private",
        Step::MailFrom => "the sender",
        Step::RcptTo => "the recipient",
        Step::Data => "asking to send",
        Step::Payload => "the letter itself",
    }
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
    /// Only attachments need it, so an engine without one can still send.
    platform: Option<Arc<crate::platform::services::PlatformService>>,
}

impl Node {
    /// One file's bytes: a store key in this node's store, or a FileRef read
    /// from the store it names — capped like every node read — with the
    /// filename it arrives under: the FileRef's own, else the key's last
    /// segment, so nobody receives `9f2c-4d1a-….pdf` named after a folder.
    fn read_source(
        platform: &std::sync::Arc<crate::platform::services::PlatformService>,
        owner: &str,
        project: &str,
        store: Option<&str>,
        source: &Value,
    ) -> Result<Option<(String, Vec<u8>)>, PipelineError> {
        if source.as_str().is_some_and(|text| text.trim().is_empty()) {
            return Ok(None);
        }
        let Some((store, rel)) = crate::pipeline::nodes::shared::project_store::open_source(platform, owner, project, source, store)?
        else {
            return Ok(None);
        };
        let rel = crate::zebfs::normalize_object_path(rel.trim_start_matches('/'))
            .map_err(|e| PipelineError::new("FW_NODE_MAIL_MESSAGE_SEND_FILE", format!("'{rel}': {e}")))?;
        let bytes = store.read_capped(&rel, "FW_NODE_MAIL_MESSAGE_SEND_FILE")?;
        let filename = source
            .get("filename")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| rel.rsplit('/').next().unwrap_or(&rel).to_string());
        Ok(Some((filename, bytes)))
    }

    /// Reads every `--file` and every `--inline` picture out of the
    /// project's store. An inline picture is placed by its id when there is
    /// an HTML body to reference it, and sent as an ordinary attachment
    /// otherwise.
    async fn resolve_attachments(
        &self,
        input: &NodeExecutionInput,
        has_html: bool,
    ) -> Result<Vec<Attachment>, PipelineError> {
        let files = listed(&self.config.file);
        if files.is_empty() && self.config.inline.is_empty() {
            return Ok(Vec::new());
        }
        if files.len() > MAX_FILES as usize {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CONFIG",
                format!("--file is given {} times; a message attaches at most {MAX_FILES}", files.len()),
            ));
        }
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let platform = self.platform.as_ref().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_FILE",
                "attachments need the platform's file store, which this engine context has not got",
            )
        })?;
        let store = self.config.store.as_deref();

        let mut out = Vec::with_capacity(files.len() + self.config.inline.len());
        for source in &files {
            if let Some((filename, bytes)) = Self::read_source(platform, owner, project, store, source)? {
                out.push(Attachment::new(filename, bytes));
            }
        }
        for (id, source) in &self.config.inline {
            let id = id.trim();
            if id.is_empty() {
                continue;
            }
            let Some((filename, bytes)) = Self::read_source(platform, owner, project, store, source)? else {
                continue;
            };
            out.push(if has_html {
                Attachment::inline(filename, id, bytes)
            } else {
                Attachment::new(filename, bytes)
            });
        }
        Ok(out)
    }

    pub fn new(
        config: Config,
        credentials: Arc<CredentialService>,
        platform: Option<Arc<crate::platform::services::PlatformService>>,
    ) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CONFIG",
                "--credential is empty; it needs the smtp credential's id",
            ));
        }
        let recipients = listed(&config.recipient);
        if recipients.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CONFIG",
                "--recipient is empty; it needs an address",
            ));
        }
        if recipients.len() > MAX_RECIPIENTS as usize {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CONFIG",
                format!("--recipient names {} addresses; one message takes at most {MAX_RECIPIENTS}", recipients.len()),
            ));
        }
        if recipients.iter().any(|r| !r.is_string()) {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_ADDRESS",
                "--recipient must be addresses (text)",
            ));
        }
        Ok(Self {
            config,
            credentials,
            platform,
        })
    }
}

/// The message's headers with every recipient in `To:` and a `Reply-To:`
/// when one is given. mailbourne writes one `To:` address; the rest are
/// added to that header in place, so the letter names everyone the envelope
/// delivers to.
fn finish_headers(raw: &[u8], recipients: &[EmailAddress], reply_to: Option<&EmailAddress>) -> Vec<u8> {
    let text = String::from_utf8_lossy(raw);
    let head_end = text.find("\r\n\r\n").unwrap_or(text.len());
    let (head, rest) = text.split_at(head_end);
    let mut lines: Vec<String> = head.split("\r\n").map(ToString::to_string).collect();
    if let Some(at) = lines.iter().position(|line| line.starts_with("To: ")) {
        let all: Vec<String> = recipients.iter().map(|r| format!("<{r}>")).collect();
        lines[at] = format!("To: {}", all.join(", "));
        if let Some(reply_to) = reply_to {
            lines.insert(at + 1, format!("Reply-To: <{reply_to}>"));
        }
    }
    let mut out = lines.join("\r\n");
    out.push_str(rest);
    out.into_bytes()
}

/// The `Message-ID` the message carries, as written.
fn message_id(raw: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(raw);
    let head = &text[..text.find("\r\n\r\n").unwrap_or(text.len())];
    head.split("\r\n").find_map(|line| line.strip_prefix("Message-ID: ").map(|id| id.trim().to_string()))
}

fn secret_str<'a>(secret: &'a Value, key: &str) -> &'a str {
    secret.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;

        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| PipelineError::new("FW_NODE_MAIL_MESSAGE_SEND_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_MAIL_MESSAGE_SEND_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;
        if credential.kind != "smtp" {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CREDENTIAL_KIND",
                format!(
                    "credential '{}' is kind '{}', expected 'smtp'",
                    credential.credential_id, credential.kind
                ),
            ));
        }
        let secret = &credential.secret;
        let host = secret_str(secret, "host").to_string();
        if host.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CREDENTIAL",
                "smtp credential has no host",
            ));
        }
        let port: u16 = secret_str(secret, "port").trim().parse().unwrap_or(587);
        let tls_mode = {
            let raw = secret_str(secret, "tls").trim().to_ascii_lowercase();
            if raw.is_empty() {
                "starttls".to_string()
            } else {
                raw
            }
        };

        // --- Resolve the message from config + payload ---
        let recipients_raw: Vec<String> =
            listed(&self.config.recipient).iter().filter_map(|r| r.as_str().map(|s| s.trim().to_string())).collect();
        let from_raw = self
            .config
            .sender
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| secret_str(secret, "from").to_string());
        let subject = self.config.subject.clone();
        let text = self.config.text.clone();
        let html = self.config.html.clone();

        // ── The message, built by mailbourne ──
        let from_address = to_mailbourne(&from_raw, "sender")?;
        let recipients = recipients_raw
            .iter()
            .map(|raw| to_mailbourne(raw, "recipient"))
            .collect::<Result<Vec<_>, _>>()?;
        let reply_to = match self.config.reply_to.as_deref().map(str::trim).filter(|r| !r.is_empty()) {
            Some(raw) => Some(to_mailbourne(raw, "reply-to")?),
            None => None,
        };
        let to_address = recipients[0].clone();
        let text = text.unwrap_or_default();
        if text.trim().is_empty() && html.as_deref().unwrap_or("").trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_MESSAGE_SEND_CONFIG",
                "one of --text or --html is required",
            ));
        }
        // A letter always carries text. When only HTML was given, the text
        // part would otherwise be empty for every reader whose client shows
        // no HTML, so say where the content went rather than nothing.
        let text = if text.trim().is_empty() {
            "This message is formatted as HTML. Open it in a mail reader that shows HTML.".to_string()
        } else {
            text
        };

        let has_html = html.as_deref().is_some_and(|h| !h.trim().is_empty());
        let attachments = self.resolve_attachments(&input, has_html).await?;
        let attached: Vec<String> = attachments.iter().map(|a| a.filename.clone()).collect();

        let hostname = from_address.domain().to_string();
        let message = compose::rich(
            &from_address,
            &to_address,
            &subject,
            &text,
            html.as_deref().filter(|h| !h.trim().is_empty()),
            &attachments,
            &format!("mail.{hostname}"),
        );
        let message = mailbourne::shared::core::Message::from_raw(finish_headers(message.raw(), &recipients, reply_to.as_ref()));
        let id = message_id(message.raw());

        // ── Over to the relay ──
        let channel = super::transport::Channel::parse(&tls_mode)?;
        // No user means no AUTH: a relay on localhost that carries mail for
        // anyone who can reach it, which is what a test sink is. A real
        // relay will refuse the envelope, and say so, which is the right
        // place to find out rather than here.
        let user = secret_str(secret, "user").to_string();
        let credentials = (!user.is_empty()).then(|| Credentials {
            user,
            password: secret_str(secret, "password").to_string(),
        });
        let envelope = Envelope { mail_from: from_address.clone(), rcpt_to: recipients.clone() };

        let outcome = super::transport::deliver_to_relay(
            &host,
            port,
            channel,
            &format!("mail.{hostname}"),
            credentials.as_ref(),
            &envelope,
            &message,
        )
        .await?;

        match outcome {
            Outcome::Delivered { .. } => {}
            // Both refusals are the node's failure, but they are different
            // failures and the message says which: a 4xx is worth retrying,
            // a 5xx never is, and a refusal at `Auth` is the credential
            // rather than the letter.
            Outcome::Deferred { at, reply } => {
                return Err(PipelineError::new(
                    "FW_NODE_MAIL_MESSAGE_SEND_DEFERRED",
                    format!(
                        "the relay said not now at {}: {} {}",
                        step_name(at),
                        reply.code,
                        reply.lines.join(" ")
                    ),
                ));
            }
            Outcome::Rejected { at, reply } => {
                return Err(PipelineError::new(
                    if at == Step::Auth {
                        "FW_NODE_MAIL_MESSAGE_SEND_AUTH"
                    } else {
                        "FW_NODE_MAIL_MESSAGE_SEND"
                    },
                    format!(
                        "the relay refused at {}: {} {}",
                        step_name(at),
                        reply.code,
                        reply.lines.join(" ")
                    ),
                ));
            }
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({
                "message": { "sent": true, "id": id, "recipient": recipients_raw, "subject": subject, "attached": attached }
            })),
            trace: vec![format!("mail.message.send: delivered to relay {host}:{port}")],
        })
    }
}

#[cfg(test)]
mod tests {

    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    use super::{Config, Node};
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::UpsertProjectCredentialRequest;

    /// A one-shot SMTP sink: speaks just enough of the protocol to accept one
    /// message, then hands back what it was given. No network, no relay, no
    /// mock of our own code — the node really speaks SMTP to something.
    async fn smtp_sink() -> (u16, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind sink");
        let port = listener.local_addr().expect("addr").port();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let (read_half, mut write) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut transcript = String::new();
            let mut line = String::new();
            let mut in_data = false;

            write
                .write_all(b"220 sink ESMTP\r\n")
                .await
                .expect("greeting");
            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                transcript.push_str(&line);
                let upper = line.trim_end().to_ascii_uppercase();
                if in_data {
                    if line.trim_end() == "." {
                        in_data = false;
                        write.write_all(b"250 Ok\r\n").await.expect("data ok");
                    }
                    continue;
                }
                let reply: &[u8] = if upper.starts_with("EHLO") || upper.starts_with("HELO") {
                    b"250-sink\r\n250 SMTPUTF8\r\n"
                } else if upper.starts_with("DATA") {
                    in_data = true;
                    b"354 End data with <CR><LF>.<CR><LF>\r\n"
                } else if upper.starts_with("QUIT") {
                    write.write_all(b"221 Bye\r\n").await.expect("bye");
                    break;
                } else {
                    b"250 Ok\r\n"
                };
                write.write_all(reply).await.expect("reply");
            }
            transcript
        });
        (port, handle)
    }

    #[tokio::test]
    async fn the_node_speaks_smtp_and_the_relay_receives_the_message() {
        let (port, sink) = smtp_sink().await;

        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "relay".to_string(),
                    title: "Test relay".to_string(),
                    kind: "smtp".to_string(),
                    secret: json!({
                        "host": "127.0.0.1",
                        "port": port.to_string(),
                        "from": "Zebflow <no-reply@example.test>",
                        "tls": "none",
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let node = Node::new(
            Config {
                credential_id: "relay".to_string(),
                // Literals here: {{ }} resolution is the engine's job, done
                // before a node runs, and tested at the engine layer. A node
                // test sees what a node sees — final config.
                recipient: json!(["sari@example.test", "budi@example.test"]),
                reply_to: Some("desk@example.test".to_string()),
                subject: "Activate your account".to_string(),
                text: Some("Welcome to researchsite.".to_string()),
                ..Default::default()
            },
            platform.credentials.clone(),
            None,
        )
        .expect("node");

        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "mail".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "kept": 1 }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "mail-smoke",
                    "request_id": "req-1",
                    "trigger": { "kind": "manual" }
                }),
                bus: None,
            })
            .await
            .expect("send");

        assert_eq!(out.payload["kept"], json!(1), "the payload is kept");
        assert_eq!(out.payload["message"]["sent"], json!(true));
        assert_eq!(out.payload["message"]["recipient"], json!(["sari@example.test", "budi@example.test"]));
        assert!(out.payload["message"]["id"].as_str().is_some_and(|id| id.starts_with('<')), "{}", out.payload["message"]);

        let transcript = sink.await.expect("sink");
        assert!(
            transcript.contains("MAIL FROM:<no-reply@example.test>"),
            "credential's From did not reach the envelope: {transcript}"
        );
        assert!(
            transcript.contains("RCPT TO:<sari@example.test>") && transcript.contains("RCPT TO:<budi@example.test>"),
            "every recipient reaches the envelope: {transcript}"
        );
        assert!(
            transcript.contains("To: <sari@example.test>, <budi@example.test>"),
            "every recipient is named in the letter: {transcript}"
        );
        assert!(transcript.contains("Reply-To: <desk@example.test>"), "reply-to is written: {transcript}");
        assert!(
            transcript.contains("Welcome to researchsite."),
            "the body did not reach the relay: {transcript}"
        );
        assert!(
            transcript.contains("Activate your account"),
            "the subject did not reach the relay: {transcript}"
        );
    }

    #[tokio::test]
    async fn a_credential_of_the_wrong_kind_is_refused_before_any_connection() {
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "not-a-relay".to_string(),
                    title: "Wrong kind".to_string(),
                    kind: "postgres".to_string(),
                    secret: json!({ "host": "127.0.0.1" }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let node = Node::new(
            Config {
                credential_id: "not-a-relay".to_string(),
                recipient: json!("ops@example.test"),
                subject: "hi".to_string(),
                text: Some("hi".to_string()),
                ..Default::default()
            },
            platform.credentials.clone(),
            None,
        )
        .expect("node");

        let err = node
            .execute_async(NodeExecutionInput {
                node_id: "mail".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "mail-kind",
                    "request_id": "req-1",
                    "trigger": { "kind": "manual" }
                }),
                bus: None,
            })
            .await
            .expect_err("wrong kind must refuse");
        assert_eq!(err.code, "FW_NODE_MAIL_MESSAGE_SEND_CREDENTIAL_KIND");
    }

    #[tokio::test]
    async fn an_unparseable_recipient_is_refused_rather_than_dialled() {
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "relay".to_string(),
                    title: "Test relay".to_string(),
                    kind: "smtp".to_string(),
                    // Port 1 would refuse instantly if we ever got that far.
                    secret: json!({ "host": "127.0.0.1", "port": "1", "from": "a@b.test", "tls": "none" }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let node = Node::new(
            Config {
                credential_id: "relay".to_string(),
                recipient: json!(["not an address"]),
                subject: "hi".to_string(),
                text: Some("hi".to_string()),
                ..Default::default()
            },
            platform.credentials.clone(),
            None,
        )
        .expect("node");

        let err = node
            .execute_async(NodeExecutionInput {
                node_id: "mail".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "mail-addr",
                    "request_id": "req-1",
                    "trigger": { "kind": "manual" }
                }),
                bus: None,
            })
            .await
            .expect_err("bad address must refuse");
        assert_eq!(err.code, "FW_NODE_MAIL_MESSAGE_SEND_ADDRESS");
    }

    /// The 0.11 words: `--recipient`, `--sender`, `--file`, `--inline`.
    #[test]
    fn the_flags_are_the_0_11_words() {
        let def = super::definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(
            flags,
            ["--credential", "--recipient", "--subject", "--text", "--html", "--sender", "--reply-to", "--file", "--inline", "--store"]
        );
        assert_eq!(super::listed(&json!(["a@example.test", ["b@example.test"], ""])), vec![json!("a@example.test"), json!("b@example.test")]);
    }
}
