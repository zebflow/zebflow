//! `n.mail.send` — outgoing mail submission through a stored `smtp` credential.
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
//! | Activation email | `\| n.mail.send --credential relay --to "{{ input.email }}" --subject "Activate your account" --text "{{ input.body }}"` |
//! | Fixed recipient | `\| n.mail.send --credential relay --to ops@example.com --subject "Backup done" --text "ok"` |
//!
//! `--to`, `--subject`, `--text`, `--html`, `--from` and `--reply-to` each
//! take a literal or `{{ expr }}` — the one resolution mechanism
//! (`docs/contracts/kinds/node-io`): `--to "{{ input.email }}"`. The engine
//! resolves before this node runs, so the config that arrives here is final.
//!
//! The output carries what was sent and to whom — never the credential.

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
use crate::pipeline::model::LayoutItem;

pub const NODE_KIND: &str = "n.mail.send";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Credential],
        title: "Send Mail".to_string(),
        description: "Sends one email through a stored smtp credential's relay. \
                      to/subject/text/html take a literal or {{ expr }}. Submission only — deliverability (SPF, DKIM, \
                      reputation) is the relay's job."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload the address and body paths resolve against."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "sent": { "type": "boolean" },
                "to": { "type": "string" },
                "subject": { "type": "string" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "credential_id": { "type": "string", "description": "ID of the smtp credential." },
                "to":       { "type": "string", "description": "Recipient — literal address or {{ expr }}." },
                "subject":  { "type": "string", "description": "Subject — literal or {{ expr }}." },
                "text":     { "type": "string", "description": "Plain-text body — literal or {{ expr }}." },
                "html":     { "type": "string", "description": "HTML body — literal or {{ expr }}. With text, sent as multipart/alternative." },
                "from":     { "type": "string", "description": "Override the credential's From." },
                "reply_to": { "type": "string", "description": "Reply-To address." },
                "attach":   { "type": "object", "description": "Displayed filename → store path or FileRef, one entry per attachment." }
            }
        }),
        dsl_flags: vec![
            crate::pipeline::model::DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "ID of the smtp credential naming the relay.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: true,
            },
            crate::pipeline::model::DslFlag {
                flag: "--to".to_string(),
                config_key: "to".to_string(),
                description: "Recipient address — literal or {{ expr }}.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: true,
            },
            crate::pipeline::model::DslFlag {
                flag: "--subject".to_string(),
                config_key: "subject".to_string(),
                description: "Subject line — literal or {{ expr }}.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: true,
            },
            crate::pipeline::model::DslFlag {
                flag: "--text".to_string(),
                config_key: "text".to_string(),
                description: "Plain-text body — literal or {{ expr }}.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--html".to_string(),
                config_key: "html".to_string(),
                description: "HTML body — literal or {{ expr }}. Given both, the mail is multipart/alternative.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--from".to_string(),
                config_key: "from".to_string(),
                description: "Override the credential's default From address.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--attach".to_string(),
                config_key: "attach".to_string(),
                description: "A file to attach: displayed name = store path or FileRef. Repeat for several. \
                    e.g. --attach \"Certificate.pdf={{ input.image.ref }}\". Leave the name empty to keep the file's own."
                    .to_string(),
                kind: crate::pipeline::model::DslFlagKind::KeyValuePairs,
                required: false,
            },
            crate::pipeline::model::DslFlag {
                flag: "--reply-to".to_string(),
                config_key: "reply_to".to_string(),
                description: "Reply-To address.".to_string(),
                kind: crate::pipeline::model::DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "SMTP Credential".to_string(), field_type: NodeFieldType::Text, help: Some("Credential of kind smtp naming the relay.".to_string()), ..Default::default() },
                NodeFieldDef { name: "to".to_string(), label: "To".to_string(), field_type: NodeFieldType::Text, help: Some("Literal address or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "subject".to_string(), label: "Subject".to_string(), field_type: NodeFieldType::Text, help: Some("Literal or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "text".to_string(), label: "Text Body".to_string(), field_type: NodeFieldType::Textarea, help: Some("Plain-text body — literal or {{ expr }}.".to_string()), ..Default::default() },
                NodeFieldDef { name: "html_section".to_string(), label: "HTML version (optional)".to_string(), field_type: NodeFieldType::Section, help: Some("An email carries the text above and, optionally, an HTML rendering of the same words. A reader's client shows one or the other — never both — so this is an addition, not a choice.".to_string()), ..Default::default() },
                NodeFieldDef { name: "html".to_string(), label: "HTML Body".to_string(), field_type: NodeFieldType::CodeEditor, language: Some("html".to_string()), rows: Some(12), help: Some("Leave empty to send plain text only. Email clients are not browsers: use tables and inline styles, and expect no JavaScript, no flexbox and no external stylesheet.".to_string()), ..Default::default() },
                NodeFieldDef { name: "attach".to_string(), label: "Attachments".to_string(), field_type: NodeFieldType::KeyValuePairs, help: Some("Each row: the name the recipient sees, and the store path or FileRef it comes from. Leave the name empty to keep the file's own — otherwise a certificate arrives called 9f2c-4d1a-….pdf.".to_string()), ..Default::default() },
                NodeFieldDef { name: "from".to_string(), label: "From".to_string(), field_type: NodeFieldType::Text, help: Some("Overrides the credential's From address.".to_string()), ..Default::default() },
                NodeFieldDef { name: "reply_to".to_string(), label: "Reply-To".to_string(), field_type: NodeFieldType::Text, help: Some("Where replies should go when that is not the From address — literal or {{ expr }}.".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("to".to_string()), LayoutItem::Field("subject".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("from".to_string()), LayoutItem::Field("reply_to".to_string())] },
            LayoutItem::Field("text".to_string()),
            LayoutItem::Field("html".to_string()),
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Confirmation after a form", r#"mail.send --credential smtp_main --to "{{ input.body.email }}" --subject "We got your message" --text "Thanks {{ input.body.name }}, we will reply within a day.""#)
                .output(serde_json::json!({ "sent": true, "to": "a@x.io", "subject": "We got your message" }))
                .note("Replaces the payload; keep what the redirect needs in `$nodes.<id>`. The credential is created by the owner in Studio → Credentials (kind smtp)."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub credential_id: String,
    pub to: String,
    pub subject: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub reply_to: Option<String>,
    /// Files to attach: displayed name → store path or FileRef. An entry
    /// with an empty name takes the file's own.
    #[serde(default)]
    pub attach: std::collections::BTreeMap<String, String>,
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
        PipelineError::new("FW_NODE_MAIL_ADDRESS", format!("{what} address '{raw}' is not valid: {e:?}"))
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
    /// Reads every attachment out of the project's store.
    ///
    /// Each value is a store path or a FileRef, resolved the way every other
    /// node resolves one. The key is the name the recipient sees; an empty
    /// key takes the file's own, so the common case stays short and nobody
    /// receives `9f2c-4d1a-….pdf`.
    async fn resolve_attachments(
        &self,
        input: &NodeExecutionInput,
    ) -> Result<Vec<Attachment>, PipelineError> {
        if self.config.attach.is_empty() {
            return Ok(Vec::new());
        }
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let platform = self.platform.as_ref().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_MAIL_ATTACH",
                "attachments need the platform's file store, which this engine context has not got",
            )
        })?;
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|e| PipelineError::new("FW_NODE_MAIL_ATTACH", e.to_string()))?;
        let zebfs = layout.open_files();

        let mut out = Vec::with_capacity(self.config.attach.len());
        for (name, source) in &self.config.attach {
            let source = source.trim();
            if source.is_empty() {
                continue;
            }
            let rel = crate::zebfs::normalize_object_path(source.trim_start_matches('/'))
                .map_err(|e| PipelineError::new("FW_NODE_MAIL_ATTACH", format!("attachment '{source}': {e}")))?;
            let object = zebfs.get(&rel).map_err(|e| {
                PipelineError::new("FW_NODE_MAIL_ATTACH", format!("attachment '{rel}': {}", e.message))
            })?;
            let filename = if name.trim().is_empty() {
                rel.rsplit('/').next().unwrap_or(&rel).to_string()
            } else {
                name.trim().to_string()
            };
            out.push(Attachment::new(filename, object.bytes));
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
                "FW_NODE_MAIL_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        if config.to.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_CONFIG",
                "config.to must not be empty",
            ));
        }
        Ok(Self {
            config,
            credentials,
            platform,
        })
    }
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
            .map_err(|err| PipelineError::new("FW_NODE_MAIL_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_MAIL_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;
        if credential.kind != "smtp" {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_CREDENTIAL_KIND",
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
                "FW_NODE_MAIL_CREDENTIAL",
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
        let to_raw = self.config.to.clone();
        let from_raw = self
            .config
            .from
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| secret_str(secret, "from").to_string());
        let subject = self.config.subject.clone();
        let text = self.config.text.clone();
        let html = self.config.html.clone();

        // ── The message, built by mailbourne ──
        let from_address = to_mailbourne(&from_raw, "sender")?;
        let to_address = to_mailbourne(&to_raw, "recipient")?;
        let text = text.unwrap_or_default();
        if text.trim().is_empty() && html.as_deref().unwrap_or("").trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_MAIL_CONFIG",
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

        let attachments = self.resolve_attachments(&input).await?;
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
        let envelope = Envelope { mail_from: from_address.clone(), rcpt_to: vec![to_address.clone()] };

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
                    "FW_NODE_MAIL_DEFERRED",
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
                        "FW_NODE_MAIL_AUTH"
                    } else {
                        "FW_NODE_MAIL_SEND"
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
            payload: json!({
                "sent": true,
                "attached": attached,
                "to": to_raw,
                "subject": subject,
            }),
            trace: vec![format!("n.mail.send: delivered to relay {host}:{port}")],
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    use super::{Config, Node};
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::{PlatformConfig, UpsertProjectCredentialRequest};
    use crate::platform::services::PlatformService;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("zebflow-mail-{name}-{now}"))
    }

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

        let mut cfg = PlatformConfig::default();
        cfg.data_root = temp_dir("send");
        let platform = Arc::new(PlatformService::from_config(cfg).expect("platform"));
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
                to: "sari@example.test".to_string(),
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
                payload: json!({}),
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

        assert_eq!(out.payload["sent"], json!(true));
        assert_eq!(out.payload["to"], json!("sari@example.test"));

        let transcript = sink.await.expect("sink");
        assert!(
            transcript.contains("MAIL FROM:<no-reply@example.test>"),
            "credential's From did not reach the envelope: {transcript}"
        );
        assert!(
            transcript.contains("RCPT TO:<sari@example.test>"),
            "the recipient did not reach the envelope: {transcript}"
        );
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
        let mut cfg = PlatformConfig::default();
        cfg.data_root = temp_dir("kind");
        let platform = Arc::new(PlatformService::from_config(cfg).expect("platform"));
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
                to: "ops@example.test".to_string(),
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
        assert_eq!(err.code, "FW_NODE_MAIL_CREDENTIAL_KIND");
    }

    #[tokio::test]
    async fn an_unparseable_recipient_is_refused_rather_than_dialled() {
        let mut cfg = PlatformConfig::default();
        cfg.data_root = temp_dir("addr");
        let platform = Arc::new(PlatformService::from_config(cfg).expect("platform"));
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
                to: "not an address".to_string(),
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
        assert_eq!(err.code, "FW_NODE_MAIL_ADDRESS");
    }
}
