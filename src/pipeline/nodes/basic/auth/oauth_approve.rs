//! `auth.oauth.approve` — completes a published MCP route's OAuth sign-in
//! from the app's own login (`docs/contracts/published-mcp.md` § `--auth
//! oauth`).
//!
//! The route's `/_oauth/authorize` sent the person to the app's `--login`
//! page with `?oauth=<ticket>&client=<name>&redirect_host=<host>`. Once the
//! app's login pipeline has checked the person and signed its own token
//! (`auth.token.create`), this node:
//!
//! 1. spends the ticket (once; ten minutes) — [`CODE_TICKET`];
//! 2. refuses unless the client name and redirect host the page showed are
//!    the ticket's — [`CODE_MISMATCH`];
//! 3. verifies the app's token with the route's `--credential` (signature
//!    and expiry) — [`CODE_TOKEN`];
//! 4. stores a two-minute authorization code holding the token's claims —
//!    [`CODE_STORE`];
//! 5. answers `oauth: { approved: true, client, redirect_host }` and the
//!    caller a `303` back to the client: `redirect_uri?code&state&iss`.
//!
//! It is an answering node, like `web.response.send`: the redirect is the
//! response. The code never enters the payload, and the record masks the
//! `Location`. A failure goes to `:error` as `oauth: { ok: false, error }`
//! when wired, so the app can show "this link has expired". No Zebflow
//! session, cookie or account is read or written.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::basic::web::response::ENVELOPE_KEY;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::platform::services::published_oauth::{Code, rules};

pub const NODE_KIND: &str = "auth.oauth.approve";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
/// A flag is empty.
pub const CODE_CONFIG: &str = "FW_NODE_AUTH_OAUTH_APPROVE_CONFIG";
/// The ticket is unknown, expired or already used.
pub const CODE_TICKET: &str = "FW_NODE_AUTH_OAUTH_APPROVE_TICKET";
/// The client or redirect host shown is not the ticket's.
pub const CODE_MISMATCH: &str = "FW_NODE_AUTH_OAUTH_APPROVE_MISMATCH";
/// The app's token does not verify with the route's credential.
pub const CODE_TOKEN: &str = "FW_NODE_AUTH_OAUTH_APPROVE_TOKEN";
/// The project's store could not keep the code.
pub const CODE_STORE: &str = "FW_NODE_AUTH_OAUTH_APPROVE_STORE";

fn flag(name: &str, key: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: true,
        value: "text".to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        span: Some("full".to_string()),
        ..Default::default()
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Credential],
        // The redirect carries the authorization code; the record masks it.
        secret_paths: vec![format!("/{ENVELOPE_KEY}/headers")],
        title: "Approve OAuth Sign-in".to_string(),
        description: "Completes the sign-in of a published MCP route with `--auth oauth`, in the app's own login pipeline, after the app has checked the person \
            and signed its token (`auth.token.create`). The route sent the person to its `--login` page with `?oauth=<ticket>&client=<name>&redirect_host=<host>`; \
            show both to the person, then pass them back: `--ticket`, `--client`, `--redirect-host`, and the app's token as `--token`. \
            The ticket is used once and lives ten minutes; the shown client and host must be the ticket's; the token must verify with the route's `--credential`. \
            Answers `oauth: { approved, client, redirect_host }` and, like `web.response.send`, answers the caller: a 303 back to the MCP client with a \
            two-minute authorization code. A failure on a wired `:error` arrives as `oauth: { ok: false, error }` — render \"this link has expired\" there. \
            Never reads or sets a Zebflow session."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `oauth` is added." }),
        output_schema: json!({ "type": "object", "properties": { "oauth": { "type": "object", "properties": {
            "approved": { "type": "boolean" },
            "client": { "type": "string", "description": "The client's name, as the person saw it." },
            "redirect_host": { "type": "string", "description": "Where the sign-in was sent." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--ticket", "ticket", "The ticket the route gave the login page — {{ input.webhook.query.oauth }}, or the form field the page posted it back in."),
            // Holding the app's token is being the person.
            DslFlag { secret: true, ..flag("--token", "token", "The app's token for the person who signed in — normally {{ input.token.access_token }}, signed with the route's --credential.") },
            flag("--client", "client", "The client name the login page showed (its `client` query value): refused unless it is the ticket's."),
            flag("--redirect-host", "redirect_host", "The host the login page said the sign-in goes to (its `redirect_host` query value): refused unless it is the ticket's."),
        ],
        fields: vec![
            field("ticket", "Ticket", "The ticket from the login page's `oauth` query value."),
            field("token", "App token", "The app's token for the signed-in person, e.g. {{ input.token.access_token }}."),
            field("client", "Client shown", "The client name the page showed."),
            field("redirect_host", "Redirect host shown", "The host the page said the sign-in goes to."),
        ],
        layout: vec![
            LayoutItem::Field("ticket".to_string()),
            LayoutItem::Field("token".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("client".to_string()), LayoutItem::Field("redirect_host".to_string())] },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl(
                "Approve after the app's login",
                r#"auth.oauth.approve --ticket "{{ input.webhook.body.oauth }}" --token "{{ input.token.access_token }}" --client "{{ input.webhook.body.client }}" --redirect-host "{{ input.webhook.body.redirect_host }}""#,
            )
            .output(json!({ "oauth": { "approved": true, "client": "Claude", "redirect_host": "claude.ai" } }))
            .note("Before it: `trigger.webhook --route /auth/login --method POST | … check the password … | auth.token.create --credential library-jwt --claim \"sub=…\" --claim \"roles={{ ['reader'] }}\"`. The caller receives a 303 to the MCP client."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub ticket: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub client: String,
    #[serde(default)]
    pub redirect_host: String,
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        Ok(Self { config, platform })
    }
}

/// Every flag, non-empty, or the one that is not.
fn given(config: &Config) -> Result<(&str, &str, &str, &str), PipelineError> {
    let pairs = [
        ("--ticket", config.ticket.trim()),
        ("--token", config.token.trim()),
        ("--client", config.client.trim()),
        ("--redirect-host", config.redirect_host.trim()),
    ];
    if let Some((name, _)) = pairs.iter().find(|(_, v)| v.is_empty()) {
        return Err(PipelineError::new(CODE_CONFIG, format!("{name} is empty; it needs a value")));
    }
    Ok((pairs[0].1, pairs[1].1, pairs[2].1, pairs[3].1))
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _, _) = metadata_scope(&input.metadata)?;
        let (ticket_secret, token, client, redirect_host) = given(&self.config)?;
        let store = &self.platform.published_oauth;
        let ticket = store
            .take_ticket(owner, project, ticket_secret)
            .map_err(|e| PipelineError::new(CODE_STORE, e.0))?
            .ok_or_else(|| PipelineError::new(CODE_TICKET, "this sign-in link is unknown, expired or already used; start again from the app"))?;
        // D1: what the person agreed to is what the ticket says.
        if client != ticket.client_name || redirect_host != ticket.redirect_host {
            return Err(PipelineError::new(
                CODE_MISMATCH,
                "the client or redirect host shown is not the sign-in's; pass the page's `client` and `redirect_host` values unchanged",
            ));
        }
        let credential = self
            .platform
            .credentials
            .get_project_credential(owner, project, &ticket.credential)
            .map_err(|e| PipelineError::new(CODE_TOKEN, e.message))?
            .filter(|c| c.kind == "jwt_signing_key")
            .ok_or_else(|| PipelineError::new(CODE_TOKEN, format!("the route's credential '{}' is not a jwt_signing_key of this project", ticket.credential)))?;
        let claims = super::sign::verify_app_token(&credential.secret, token).map_err(|err| match err {
            super::sign::VerifyError::Key(key) => PipelineError::new(CODE_TOKEN, key.to_string()),
            super::sign::VerifyError::Token(reason) => {
                PipelineError::new(CODE_TOKEN, format!("--token does not verify with credential '{}': {reason}", ticket.credential))
            }
        })?;
        let code = Code {
            client_id: ticket.client_id.clone(),
            redirect_uri: ticket.redirect_uri.clone(),
            challenge: ticket.challenge.clone(),
            resource: ticket.resource.clone(),
            issuer: ticket.issuer.clone(),
            scope: ticket.scope.clone(),
            claims,
            family: uuid::Uuid::new_v4().simple().to_string(),
        };
        let code_secret = store.issue_code(owner, project, &ticket.route, &code).map_err(|e| PipelineError::new(CODE_STORE, e.0))?;
        let mut pairs = vec![("code", code_secret.as_str())];
        if let Some(state) = ticket.state.as_deref() {
            pairs.push(("state", state));
        }
        pairs.push(("iss", ticket.issuer.as_str()));
        let location = rules::with_query(&ticket.redirect_uri, &pairs);
        let envelope: Value = json!({
            "status": 303,
            "headers": [["Location", location], ["Cache-Control", "no-store"]],
        });
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(
                &input.payload,
                json!({
                    "oauth": { "approved": true, "client": ticket.client_name, "redirect_host": ticket.redirect_host },
                    ENVELOPE_KEY: envelope,
                }),
            ),
            trace: vec![format!("auth.oauth.approve: route {} client {}", ticket.route, ticket.client_id)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_flag_is_required_and_the_token_is_secret() {
        let def = definition();
        assert!(def.dsl_flags.iter().all(|f| f.required));
        assert!(def.dsl_flags.iter().find(|f| f.flag == "--token").is_some_and(|f| f.secret));
        let err = given(&Config { ticket: "t".into(), token: uuid::Uuid::new_v4().to_string(), client: " ".into(), redirect_host: "h".into() }).unwrap_err();
        assert_eq!(err.code, CODE_CONFIG);
        assert!(err.message.contains("--client"));
    }
}
