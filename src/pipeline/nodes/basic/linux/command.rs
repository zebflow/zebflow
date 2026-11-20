//! `linux.command.run` — runs one command line on a Linux machine over SSH, as
//! the user an `ssh` credential names (`docs/contracts/linux-control.md`).
//!
//! # Pipeline position
//!
//! Anywhere a pipeline needs a machine: build a library, run a job, write a
//! file (`--run "cat > notes.txt" --stdin "{{ … }}"`), manage a service
//! (`systemctl --user restart site-a`). What the command may touch is that
//! Linux user's own permissions on the machine.
//!
//! # DSL
//! ```text
//! | linux.command.run --credential compute-1 --run "bun --version"
//! ```
//!
//! # Input/output
//! - **Output `out`:** the payload plus `command: { ok, exit_code, stdout,
//!   stderr, duration_ms, truncated }`. A non-zero exit is `ok: false`.
//! - **Failure:** a refused credential or line, an unreachable machine, a
//!   changed host key, or the time limit.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::ssh::{self, CREDENTIAL_KIND, Failure, Target};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeCapability, NodeExample, NodeFieldDataSource, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::units;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

pub const NODE_KIND: &str = "linux.command.run";
/// The machine could not be used: unreachable, a failed login, a timeout.
pub const CODE: &str = "FW_NODE_LINUX_COMMAND_RUN";
/// The node or its credential is set up wrong.
pub const CONFIG_CODE: &str = "FW_NODE_LINUX_COMMAND_RUN_CONFIG";
/// The line or the machine was refused: not a listed program, a changed host key.
pub const REFUSED_CODE: &str = "FW_NODE_LINUX_COMMAND_RUN_REFUSED";

const DEFAULT_TIMEOUT: &str = "60s";
const MAX_TIMEOUT: Duration = Duration::from_secs(3600);
const DEFAULT_MAX_OUTPUT: u64 = 1024 * 1024;
const MAX_OUTPUT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The `ssh` credential: which machine, as which user.
    #[serde(default)]
    pub credential_id: String,
    /// The command line the user's shell runs.
    #[serde(default)]
    pub run: String,
    /// Text sent to the command's standard input.
    #[serde(default)]
    pub stdin: Option<String>,
    /// How long the run may take: `60s`, `5m`.
    #[serde(default)]
    pub timeout: Option<String>,
    /// Bytes kept of each of stdout and stderr: `1MB`.
    #[serde(default)]
    pub max_output: Option<String>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
    timeout: Duration,
    max_output: usize,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--credential is empty; it needs an ssh credential's id"));
        }
        let timeout = units::duration(config.timeout.as_deref().unwrap_or(DEFAULT_TIMEOUT), "--timeout", CONFIG_CODE)?;
        if timeout.is_zero() || timeout > MAX_TIMEOUT {
            return Err(PipelineError::new(CONFIG_CODE, "--timeout is from 1s to 1h"));
        }
        let max_output = match config.max_output.as_deref() {
            Some(raw) if !raw.trim().is_empty() => units::size(raw, "--max-output", CONFIG_CODE)?,
            _ => DEFAULT_MAX_OUTPUT,
        };
        if max_output == 0 || max_output > MAX_OUTPUT {
            return Err(PipelineError::new(CONFIG_CODE, "--max-output is from 1B to 16MB"));
        }
        Ok(Self { config, credentials, timeout, max_output: max_output as usize })
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Run a Linux Command".to_string(),
        description: "Runs one command line on a Linux machine over SSH, as the user an `ssh` credential names: a server or a \
            long-running container the office can reach. The credential pins the machine's host key, so a machine answering with \
            another key is refused, and may list the programs a line may start with. What the command may touch is that Linux \
            user's own permissions. `--stdin` sends text to the command (a file is written with `--run \"cat > notes.txt\"`). \
            `out` adds `command: { ok, exit_code, stdout, stderr, duration_ms, truncated }`; a non-zero exit is `ok: false`, \
            while a refused line, an unreachable machine, a changed host key or the `--timeout` fails the node."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string()],
        output_schema: json!({
            "description": "The payload plus `command`: what the line printed and how it exited.",
            "properties": { "command": { "properties": {
                "ok": { "type": "boolean" }, "exit_code": { "type": "integer" }, "stdout": { "type": "string" },
                "stderr": { "type": "string" }, "duration_ms": { "type": "integer" }, "truncated": { "type": "boolean" }
            } } }
        }),
        config_schema: json!({
            "type": "object",
            "required": ["credential_id", "run"],
            "properties": {
                "credential_id": { "type": "string", "description": "The ssh credential." },
                "run": { "type": "string", "description": "The command line." },
                "stdin": { "type": "string", "description": "Text for the command's standard input." },
                "timeout": { "type": "string", "description": "60s by default, at most 1h." },
                "max_output": { "type": "string", "description": "Bytes kept of stdout and of stderr; 1MB by default, at most 16MB." }
            }
        }),
        dsl_flags: vec![
            DslFlag { flag: "--credential".to_string(), config_key: "credential_id".to_string(), description: "The ssh credential: which machine, as which user.".to_string(), kind: DslFlagKind::Scalar, required: true, value: "text".to_string(), ..Default::default() },
            DslFlag { flag: "--run".to_string(), config_key: "run".to_string(), description: "The command line the user's shell runs.".to_string(), kind: DslFlagKind::Scalar, required: true, value: "expression".to_string(), ..Default::default() },
            DslFlag { flag: "--stdin".to_string(), config_key: "stdin".to_string(), description: "Text sent to the command's standard input.".to_string(), kind: DslFlagKind::Scalar, required: false, value: "expression".to_string(), ..Default::default() },
            DslFlag { flag: "--timeout".to_string(), config_key: "timeout".to_string(), description: "How long the run may take: 60s by default, at most 1h.".to_string(), kind: DslFlagKind::Scalar, required: false, value: "duration".to_string(), ..Default::default() },
            DslFlag { flag: "--max-output".to_string(), config_key: "max_output".to_string(), description: "Bytes kept of stdout and of stderr: 1MB by default, at most 16MB.".to_string(), kind: DslFlagKind::Scalar, required: false, value: "size".to_string(), ..Default::default() },
        ],
        fields: vec![
            NodeFieldDef { name: "credential_id".to_string(), label: "Machine".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsByKind(CREDENTIAL_KIND.to_string())), help: Some("The ssh credential.".to_string()), ..Default::default() },
            NodeFieldDef { name: "run".to_string(), label: "Command".to_string(), field_type: NodeFieldType::Text, help: Some("The command line, run by the user's shell.".to_string()), ..Default::default() },
            NodeFieldDef { name: "stdin".to_string(), label: "Input".to_string(), field_type: NodeFieldType::Text, help: Some("Text sent to the command's standard input.".to_string()), ..Default::default() },
            NodeFieldDef { name: "timeout".to_string(), label: "Time limit".to_string(), field_type: NodeFieldType::Text, help: Some("60s by default, at most 1h.".to_string()), ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("Ask a machine which Bun it has", r#"linux.command.run --credential compute-1 --run "bun --version""#)
                .output(json!({ "command": { "ok": true, "exit_code": 0, "stdout": "1.4.4\n", "stderr": "", "duration_ms": 412, "truncated": false } })),
            NodeExample::dsl("Write a file from the payload", r#"linux.command.run --credential compute-1 --run "cat > notes.txt" --stdin "{{ input.webhook.body.text }}""#),
        ],
        ..Default::default()
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }

    fn input_pins(&self) -> &'static [&'static str] {
        &["in"]
    }

    fn output_pins(&self) -> &'static [&'static str] {
        &["out"]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| PipelineError::new(CODE, err.message))?
            .ok_or_else(|| PipelineError::new(CONFIG_CODE, format!("credential '{}' not found", self.config.credential_id)))?;
        if credential.kind != CREDENTIAL_KIND {
            return Err(PipelineError::new(
                CONFIG_CODE,
                format!("credential '{}' is a {} credential, not {CREDENTIAL_KIND}", credential.credential_id, credential.kind),
            ));
        }
        let target = Target::from_secret(&credential.credential_id, &credential.secret).map_err(|err| PipelineError::new(CONFIG_CODE, err))?;
        let line = target.line(&self.config.run).map_err(|err| PipelineError::new(REFUSED_CODE, err))?;
        let ran = ssh::run(&target, &line, self.config.stdin.as_deref(), self.timeout, self.max_output)
            .await
            .map_err(|failure| match failure {
                Failure::HostKey => PipelineError::new(
                    REFUSED_CODE,
                    format!("{}:{} answered with a host key that is not the one credential '{}' pins", target.host, target.port, credential.credential_id),
                ),
                Failure::Timeout => PipelineError::new(CODE, format!("the command ran past --timeout {}", units::describe_duration(self.timeout))),
                Failure::Connect(reason) => PipelineError::new(CODE, format!("{}@{}:{}: {reason}", target.user, target.host, target.port)),
            })?;
        let answer = json!({
            "ok": ran.exit_code == 0,
            "exit_code": ran.exit_code,
            "stdout": String::from_utf8_lossy(&ran.stdout),
            "stderr": String::from_utf8_lossy(&ran.stderr),
            "duration_ms": ran.duration.as_millis() as u64,
            "truncated": ran.truncated,
        });
        Ok(NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: with_answer(input.payload, json!({ "command": answer })),
            trace: vec![format!("linux.command.run: {}@{} exited {}", target.user, target.host, ran.exit_code)],
        })
    }
}
