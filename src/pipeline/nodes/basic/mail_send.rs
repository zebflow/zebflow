//! `n.mail.send` — outgoing mail submission through a stored `smtp` credential.
//!
//! Submission only: this node hands one message to a relay the credential
//! names (port 587 STARTTLS by default). It is deliberately not a mail
//! server — no queueing, no retries, no DKIM. Deliverability belongs to the
//! relay; when Mailbourne exists it becomes one more relay this node can
//! point at, and nothing here changes.
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
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials as SmtpCredentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

use super::util::metadata_scope;
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
                "reply_to": { "type": "string", "description": "Reply-To address." }
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
                NodeFieldDef { name: "html".to_string(), label: "HTML Body".to_string(), field_type: NodeFieldType::Textarea, help: Some("HTML body — with a text body, sent as multipart/alternative.".to_string()), ..Default::default() },
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
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
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
        })
    }
}

fn parse_mailbox(value: &str, what: &str) -> Result<Mailbox, PipelineError> {
    value.parse::<Mailbox>().map_err(|err| {
        PipelineError::new(
            "FW_NODE_MAIL_ADDRESS",
            format!("{what} address '{value}' is not a valid mailbox: {err}"),
        )
    })
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
        let to = parse_mailbox(&to_raw, "recipient")?;
        let from_raw = self
            .config
            .from
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| secret_str(secret, "from").to_string());
        let from = parse_mailbox(&from_raw, "from")?;
        let subject = self.config.subject.clone();
        let text = self.config.text.clone();
        let html = self.config.html.clone();

        let mut builder = Message::builder()
            .from(from)
            .to(to.clone())
            .subject(subject.clone());
        if let Some(reply_to) = self.config.reply_to.clone().filter(|s| !s.trim().is_empty()) {
            builder = builder.reply_to(parse_mailbox(&reply_to, "reply-to")?);
        }
        let message = match (text, html) {
            (Some(text), Some(html)) => builder
                .multipart(MultiPart::alternative_plain_html(text, html)),
            (None, Some(html)) => builder
                .header(lettre::message::header::ContentType::TEXT_HTML)
                .body(html),
            (Some(text), None) => builder.body(text),
            (None, None) => {
                return Err(PipelineError::new(
                    "FW_NODE_MAIL_CONFIG",
                    "one of --text or --html is required",
                ));
            }
        }
        .map_err(|err| {
            PipelineError::new("FW_NODE_MAIL_BUILD", format!("building message: {err}"))
        })?;

        // --- Transport per the credential's TLS mode ---
        let mut transport = match tls_mode.as_str() {
            // Port 587: plaintext connect, upgrade via STARTTLS, refuse to
            // continue without it.
            "starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host).map_err(
                |err| PipelineError::new("FW_NODE_MAIL_TRANSPORT", err.to_string()),
            )?,
            // Port 465: TLS from the first byte.
            "tls" => AsyncSmtpTransport::<Tokio1Executor>::relay(&host)
                .map_err(|err| PipelineError::new("FW_NODE_MAIL_TRANSPORT", err.to_string()))?,
            // Encrypted, but the certificate is not checked.
            //
            // This exists for one real case: a mail server you run yourself,
            // before it has a certificate a public authority signed — a fresh
            // mailbourne, for instance, whose STARTTLS is self-signed on first
            // boot. Such a server refuses to accept a password over a plain
            // connection, quite rightly, so "none" cannot reach it and the
            // choice would otherwise be between no encryption and no test.
            //
            // What it costs: the traffic is encrypted against a passive
            // listener, and defenceless against an active one, who can present
            // any certificate and read the password. Use it on a private
            // network — a localhost, a tunnel, a VPN — and replace it with
            // `starttls` the moment the server has a real certificate.
            "starttls-insecure" => {
                // Both waivers, because they fail separately: a self-signed
                // certificate is an invalid *cert*, and reaching that server
                // through a tunnel or an IP makes the name not match either.
                let parameters = TlsParameters::builder(host.clone())
                    .dangerous_accept_invalid_certs(true)
                    .dangerous_accept_invalid_hostnames(true)
                    .build()
                    .map_err(|err| {
                        PipelineError::new("FW_NODE_MAIL_TRANSPORT", err.to_string())
                    })?;
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
                    .tls(Tls::Required(parameters))
            }
            // Test sinks only — a localhost mailpit/smtp4dev. Never a real relay.
            "none" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host),
            other => {
                return Err(PipelineError::new(
                    "FW_NODE_MAIL_CREDENTIAL",
                    format!(
                        "unknown tls mode '{other}': use starttls, tls, starttls-insecure, or none"
                    ),
                ));
            }
        }
        .port(port);
        let user = secret_str(secret, "user");
        let password = secret_str(secret, "password");
        if !user.is_empty() {
            transport = transport.credentials(SmtpCredentials::new(
                user.to_string(),
                password.to_string(),
            ));
        }
        let transport = transport.build();

        transport.send(message).await.map_err(|err| {
            PipelineError::new("FW_NODE_MAIL_SEND", format!("smtp send failed: {err}"))
        })?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "sent": true,
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
