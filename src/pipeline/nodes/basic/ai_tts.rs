//! `n.ai.tts` — synthesize speech from text using pluggable TTS providers.
//!
//! First stable provider:
//! - `piper` via local Python runtime
//!
//! Local provider assets are resolved from Zebflow FS using relative paths stored
//! in the selected credential secret.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::{eval_deno_expr, metadata_scope};
use crate::language::LanguageEngine;
use crate::pipeline::PipelineError;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeDefinition, NodeFieldDataSource, NodeFieldDef,
    NodeFieldType, SelectOptionDef,
};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::platform::services::{CredentialService, PlatformService};

pub const NODE_KIND: &str = "n.ai.tts";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

const PIPER_BRIDGE_SCRIPT: &str = r#"
import base64
import io
import json
import sys
import wave

import numpy as np
from piper.config import SynthesisConfig
from piper.voice import PiperVoice

def fail(message: str) -> None:
    sys.stderr.write(message + "\n")
    sys.exit(1)

try:
    req = json.load(sys.stdin)
    load_kwargs = {
        "model_path": req["model_path"],
        "config_path": req["config_path"],
        "use_cuda": False,
    }
    if req.get("espeak_data_dir"):
        load_kwargs["espeak_data_dir"] = req["espeak_data_dir"]

    voice = PiperVoice.load(**load_kwargs)

    syn_config = SynthesisConfig(
        speaker_id=req.get("speaker"),
        length_scale=req.get("length_scale"),
        volume=req.get("volume", 1.0),
    )

    chunks = list(voice.synthesize(req["text"], syn_config=syn_config))
    if chunks:
        audio = np.concatenate([chunk.audio_int16_array for chunk in chunks])
    else:
        audio = np.array([], dtype=np.int16)

    wav_buffer = io.BytesIO()
    with wave.open(wav_buffer, "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)
        wf.setframerate(voice.config.sample_rate)
        wf.writeframes(audio.astype("<i2").tobytes())

    wav_bytes = wav_buffer.getvalue()
    duration_ms = round((len(audio) / voice.config.sample_rate) * 1000) if voice.config.sample_rate else 0
    payload = {
        "ok": True,
        "sample_rate": voice.config.sample_rate,
        "samples": int(len(audio)),
        "duration_ms": int(duration_ms),
        "bytes": int(len(wav_bytes)),
        "audio_blob_base64": base64.b64encode(wav_bytes).decode("ascii"),
    }
    json.dump(payload, sys.stdout)
except Exception as exc:  # noqa: BLE001
    fail(str(exc))
"#;

fn default_provider() -> String {
    "piper".to_string()
}

fn default_return_mode() -> ReturnMode {
    ReturnMode::Both
}

fn default_speed() -> f32 {
    1.0
}

fn default_volume() -> f32 {
    1.0
}

fn default_lipsync_mode() -> LipSyncMode {
    LipSyncMode::None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReturnMode {
    File,
    Blob,
    Both,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LipSyncMode {
    None,
    Basic,
    TimedWords,
    AudioGuided,
    AudioSegmented,
}

impl LipSyncMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Basic => "basic",
            Self::TimedWords => "timed_words",
            Self::AudioGuided => "audio_guided",
            Self::AudioSegmented => "audio_segmented",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub credential_id: String,
    pub text_expr: String,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub output_path_expr: Option<String>,
    #[serde(default = "default_return_mode")]
    pub return_mode: ReturnMode,
    #[serde(default)]
    pub speaker: Option<i64>,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default = "default_lipsync_mode")]
    pub lipsync_mode: LipSyncMode,
    #[serde(default)]
    pub lipsync_expr: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            credential_id: String::new(),
            text_expr: String::new(),
            output_path: None,
            output_path_expr: None,
            return_mode: default_return_mode(),
            speaker: None,
            speed: default_speed(),
            volume: default_volume(),
            lipsync_mode: default_lipsync_mode(),
            lipsync_expr: None,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PiperCredentialSecret {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    model_file: Option<String>,
    #[serde(default)]
    config_file: Option<String>,
    #[serde(default)]
    espeak_data_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PiperBridgeResult {
    sample_rate: i64,
    samples: usize,
    duration_ms: u64,
    bytes: usize,
    audio_blob_base64: String,
}

#[derive(Debug, Clone)]
struct TimedWord {
    word: String,
    weight: f32,
    pause_after_ms: u64,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "AI TTS".to_string(),
        description: "Synthesize speech from text. First stable provider is local Piper. Model, config, and espeak data are resolved from the selected credential under Zebflow FS.".to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Current payload. Use --text-expr to choose what text to speak."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "audio": {
                    "type": "object",
                    "properties": {
                        "provider": { "type": "string" },
                        "format": { "type": "string" },
                        "mime_type": { "type": "string" },
                        "path": { "type": ["string", "null"] },
                        "url": { "type": ["string", "null"] },
                        "sample_rate": { "type": "integer" },
                        "samples": { "type": "integer" },
                        "bytes": { "type": "integer" },
                        "duration_ms": { "type": "integer" },
                        "credential_id": { "type": "string" }
                    }
                },
                "audio_blob_base64": { "type": ["string", "null"] },
                "word_timings": { "type": ["array", "null"] },
                "lipsync": { "type": ["object", "null"] }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            DslFlag {
                flag: "--provider".to_string(),
                config_key: "provider".to_string(),
                description: "TTS provider. First stable provider: piper.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "Project credential that binds the provider runtime and local model files.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--text-expr".to_string(),
                config_key: "text_expr".to_string(),
                description: "Expression that resolves to the text to synthesize.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--output-path".to_string(),
                config_key: "output_path".to_string(),
                description: "Zebflow FS output object path, for example audio/demo.wav.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output-path-expr".to_string(),
                config_key: "output_path_expr".to_string(),
                description: "Expression that resolves to the Zebflow FS output object path.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--return".to_string(),
                config_key: "return_mode".to_string(),
                description: "Return mode: file, blob, or both.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--speaker".to_string(),
                config_key: "speaker".to_string(),
                description: "Optional speaker id for multi-speaker voices.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--speed".to_string(),
                config_key: "speed".to_string(),
                description: "Playback speed factor. 1.0 = normal. Greater is faster.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--volume".to_string(),
                config_key: "volume".to_string(),
                description: "Audio volume multiplier.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--lipsync".to_string(),
                config_key: "lipsync_mode".to_string(),
                description: "Optional lipsync mode: none, basic, timed_words, audio_guided, or audio_segmented.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--lipsync-expr".to_string(),
                config_key: "lipsync_expr".to_string(),
                description: "Expression alternative for lipsync mode. Overrides --lipsync when set.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "provider".to_string(),
                label: "Provider".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![SelectOptionDef {
                    value: "piper".to_string(),
                    label: "Piper".to_string(),
                }],
                default_value: Some(json!("piper")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsAll),
                help: Some("Credential with provider binding and local private-file references.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "return_mode".to_string(),
                label: "Return".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "file".to_string(), label: "File".to_string() },
                    SelectOptionDef { value: "blob".to_string(), label: "Blob".to_string() },
                    SelectOptionDef { value: "both".to_string(), label: "Both".to_string() },
                ],
                default_value: Some(json!("both")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_path".to_string(),
                label: "Output Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("audio/arin-demo.wav".to_string()),
                help: Some("Private-relative output file path. Required for file/both return mode.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_path_expr".to_string(),
                label: "Output Path Expr".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("'audio/' + $input.slug + '.wav'".to_string()),
                help: Some("Expression alternative to Output Path. Overrides output_path when set.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "speaker".to_string(),
                label: "Speaker".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Optional speaker id for multi-speaker voices.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "speed".to_string(),
                label: "Speed".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(1.0)),
                help: Some("Speed factor. 1.0 = normal. Greater is faster.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "volume".to_string(),
                label: "Volume".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(1.0)),
                help: Some("Audio volume multiplier.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "lipsync_mode".to_string(),
                label: "Lipsync".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "none".to_string(), label: "None".to_string() },
                    SelectOptionDef { value: "basic".to_string(), label: "Basic".to_string() },
                    SelectOptionDef { value: "timed_words".to_string(), label: "Timed Words".to_string() },
                    SelectOptionDef { value: "audio_guided".to_string(), label: "Audio Guided".to_string() },
                    SelectOptionDef { value: "audio_segmented".to_string(), label: "Audio Segmented".to_string() },
                ],
                default_value: Some(json!("none")),
                help: Some("Optional cheap lipsync metadata strategy.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "lipsync_expr".to_string(),
                label: "Lipsync Expr".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("$input.lipsync_method || 'none'".to_string()),
                help: Some("Expression alternative for lipsync mode. Overrides Lipsync when set.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "text_expr".to_string(),
                label: "Text Expr".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(4),
                placeholder: Some("$input.text".to_string()),
                help: Some("Expression that resolves to the text to synthesize.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("provider".to_string()),
                    LayoutItem::Field("credential_id".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("return_mode".to_string()),
                    LayoutItem::Field("output_path".to_string()),
                ],
            },
            LayoutItem::Field("output_path_expr".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("speaker".to_string()),
                    LayoutItem::Field("speed".to_string()),
                    LayoutItem::Field("volume".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("lipsync_mode".to_string()),
                    LayoutItem::Field("lipsync_expr".to_string()),
                ],
            },
            LayoutItem::Field("text_expr".to_string()),
        ],
        ..Default::default()
    }
}

pub struct Node {
    config: Config,
    credentials: Option<Arc<CredentialService>>,
    platform: Option<Arc<PlatformService>>,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
        language: Arc<dyn LanguageEngine>,
    ) -> Self {
        Self {
            config,
            credentials,
            platform,
            language,
        }
    }
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
        let (owner, project, _, _) = metadata_scope(&input.metadata)?;
        let platform = self.platform.as_ref().ok_or_else(|| {
            PipelineError::new(
                "AI_TTS_PLATFORM",
                "platform service is not configured on this framework engine",
            )
        })?;
        let credentials = self.credentials.as_ref().ok_or_else(|| {
            PipelineError::new(
                "AI_TTS_CREDENTIALS",
                "credential service is not configured on this framework engine",
            )
        })?;
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("AI_TTS_LAYOUT", err.to_string()))?;

        let provider = self.config.provider.trim().to_lowercase();
        if provider != "piper" {
            return Err(PipelineError::new(
                "AI_TTS_PROVIDER",
                format!("unsupported provider '{provider}' — expected piper"),
            ));
        }

        let text_value = eval_deno_expr(
            self.language.as_ref(),
            &self.config.text_expr,
            &input.payload,
            &input.metadata,
        )?;
        let text = value_as_non_empty_string("text_expr", &text_value)?;

        let credential_id = self.config.credential_id.trim();
        if credential_id.is_empty() {
            return Err(PipelineError::new(
                "AI_TTS_CREDENTIAL",
                "credential_id must not be empty",
            ));
        }

        let credential = credentials
            .get_project_credential(owner, project, credential_id)
            .map_err(|err| PipelineError::new("AI_TTS_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "AI_TTS_CREDENTIAL",
                    format!("credential '{credential_id}' not found"),
                )
            })?;
        if credential.kind != "tts" {
            return Err(PipelineError::new(
                "AI_TTS_CREDENTIAL_KIND",
                format!(
                    "credential '{credential_id}' must have kind 'tts', got '{}'",
                    credential.kind
                ),
            ));
        }

        let secret: PiperCredentialSecret = serde_json::from_value(credential.secret.clone())
            .map_err(|err| {
                PipelineError::new(
                    "AI_TTS_CREDENTIAL_SECRET",
                    format!("invalid tts credential secret: {err}"),
                )
            })?;
        if let Some(secret_provider) = secret.provider.as_deref() {
            let secret_provider = secret_provider.trim().to_lowercase();
            if !secret_provider.is_empty() && secret_provider != provider {
                return Err(PipelineError::new(
                    "AI_TTS_PROVIDER_MISMATCH",
                    format!(
                        "node provider '{provider}' does not match credential provider '{secret_provider}'"
                    ),
                ));
            }
        }

        let files_root = layout.files_dir.clone();
        let model_abs = resolve_zebfs_abs(
            &files_root,
            required_secret_str(&secret.model_file, "model_file")?,
        )?;
        let config_abs = resolve_zebfs_abs(
            &files_root,
            required_secret_str(&secret.config_file, "config_file")?,
        )?;
        let espeak_abs = secret
            .espeak_data_dir
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| resolve_zebfs_abs(&files_root, value))
            .transpose()?;
        if !model_abs.is_file() {
            return Err(PipelineError::new(
                "AI_TTS_MODEL",
                format!("model file '{}' does not exist", model_abs.display()),
            ));
        }
        if !config_abs.is_file() {
            return Err(PipelineError::new(
                "AI_TTS_CONFIG",
                format!("config file '{}' does not exist", config_abs.display()),
            ));
        }
        if let Some(espeak_abs) = espeak_abs.as_ref() {
            if !espeak_abs.is_dir() {
                return Err(PipelineError::new(
                    "AI_TTS_ESPEAK",
                    format!("espeak data dir '{}' does not exist", espeak_abs.display()),
                ));
            }
        }

        let speaker = self
            .config
            .speaker
            .map(|value| i32::try_from(value).ok())
            .flatten();
        let length_scale = speed_to_length_scale(self.config.speed)?;
        let lipsync_mode = resolve_lipsync_mode(
            self.config.lipsync_expr.as_deref(),
            self.config.lipsync_mode,
            &input.payload,
            &input.metadata,
            self.language.as_ref(),
        )?;

        let bridge_result = run_piper_bridge(&PiperBridgeRequest {
            model_path: model_abs,
            config_path: config_abs,
            espeak_data_dir: espeak_abs,
            text: text.clone(),
            speaker,
            length_scale,
            volume: self.config.volume,
        })?;

        let wav_bytes = BASE64_STANDARD
            .decode(bridge_result.audio_blob_base64.as_bytes())
            .map_err(|err| {
                PipelineError::new("AI_TTS_BLOB", format!("failed to decode audio blob: {err}"))
            })?;
        let (word_timings, lipsync_payload) = build_lipsync_payload(
            lipsync_mode,
            &text,
            bridge_result.duration_ms,
            bridge_result.sample_rate,
            &wav_bytes,
        )?;

        let needs_file = matches!(self.config.return_mode, ReturnMode::File | ReturnMode::Both);
        let needs_blob = matches!(self.config.return_mode, ReturnMode::Blob | ReturnMode::Both);

        let (file_rel_path, file_url) = if needs_file {
            let output_rel = resolve_output_rel_path(
                self.config.output_path.as_deref(),
                self.config.output_path_expr.as_deref(),
                &input.payload,
                &input.metadata,
                self.language.as_ref(),
            )?;
            let final_rel = normalize_audio_output_rel_path(&output_rel)?;
            let abs_path = layout.files_dir.join(&final_rel);
            if let Some(parent) = abs_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    PipelineError::new(
                        "AI_TTS_FILE",
                        format!("failed to create output directory: {err}"),
                    )
                })?;
            }
            fs::write(&abs_path, &wav_bytes).map_err(|err| {
                PipelineError::new("AI_TTS_FILE", format!("failed to write wav file: {err}"))
            })?;
            (
                Some(final_rel.clone()),
                Some(format!("/fs/{owner}/{project}/{final_rel}")),
            )
        } else {
            (None, None)
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "audio": {
                    "provider": provider,
                    "format": "wav",
                    "mime_type": "audio/wav",
                    "path": file_rel_path,
                    "url": file_url,
                    "sample_rate": bridge_result.sample_rate,
                    "samples": bridge_result.samples,
                    "bytes": bridge_result.bytes,
                    "duration_ms": bridge_result.duration_ms,
                    "credential_id": credential_id,
                },
                "audio_blob_base64": if needs_blob { Value::String(bridge_result.audio_blob_base64) } else { Value::Null },
                "word_timings": word_timings,
                "lipsync": lipsync_payload
            }),
            trace: vec![format!(
                "node_kind={NODE_KIND} provider=piper credential={credential_id} lipsync={}",
                lipsync_mode.as_str()
            )],
        })
    }
}

#[derive(Debug, Serialize)]
struct PiperBridgeRequest {
    model_path: PathBuf,
    config_path: PathBuf,
    espeak_data_dir: Option<PathBuf>,
    text: String,
    speaker: Option<i32>,
    length_scale: Option<f32>,
    volume: f32,
}

fn run_piper_bridge(req: &PiperBridgeRequest) -> Result<PiperBridgeResult, PipelineError> {
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(PIPER_BRIDGE_SCRIPT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            PipelineError::new("AI_TTS_PIPER", format!("failed to start python3: {err}"))
        })?;

    {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err(PipelineError::new(
                "AI_TTS_PIPER",
                "failed to open python stdin",
            ));
        };
        let payload = serde_json::to_vec(req).map_err(|err| {
            PipelineError::new(
                "AI_TTS_PIPER",
                format!("failed to serialize request: {err}"),
            )
        })?;
        stdin.write_all(&payload).map_err(|err| {
            PipelineError::new(
                "AI_TTS_PIPER",
                format!("failed to write python stdin: {err}"),
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|err| {
        PipelineError::new("AI_TTS_PIPER", format!("failed waiting for python: {err}"))
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("python exited with status {}", output.status)
        };
        return Err(PipelineError::new("AI_TTS_PIPER", detail));
    }

    serde_json::from_slice::<PiperBridgeResult>(&output.stdout).map_err(|err| {
        PipelineError::new(
            "AI_TTS_PIPER",
            format!("invalid python result payload: {err}"),
        )
    })
}

fn required_secret_str<'a>(
    value: &'a Option<String>,
    field: &str,
) -> Result<&'a str, PipelineError> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PipelineError::new(
                "AI_TTS_CREDENTIAL_SECRET",
                format!("tts credential secret must include non-empty '{field}'"),
            )
        })
}

fn speed_to_length_scale(speed: f32) -> Result<Option<f32>, PipelineError> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(PipelineError::new(
            "AI_TTS_SPEED",
            "speed must be a finite number greater than 0",
        ));
    }
    if (speed - 1.0).abs() < f32::EPSILON {
        Ok(None)
    } else {
        Ok(Some(1.0 / speed))
    }
}

fn value_as_non_empty_string(field: &str, value: &Value) -> Result<String, PipelineError> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        _ => Err(PipelineError::new(
            "AI_TTS_TEXT",
            format!("{field} must resolve to a non-empty string"),
        )),
    }
}

fn parse_lipsync_mode(raw: &str) -> Result<LipSyncMode, PipelineError> {
    match raw.trim().to_lowercase().as_str() {
        "" | "none" => Ok(LipSyncMode::None),
        "basic" | "word_to_vowel" => Ok(LipSyncMode::Basic),
        "timed_words" | "timed" => Ok(LipSyncMode::TimedWords),
        "audio_guided" | "audio" => Ok(LipSyncMode::AudioGuided),
        "audio_segmented" | "segmented" => Ok(LipSyncMode::AudioSegmented),
        other => Err(PipelineError::new(
            "AI_TTS_LIPSYNC",
            format!(
                "unsupported lipsync mode '{other}' — expected none, basic, timed_words, audio_guided, or audio_segmented"
            ),
        )),
    }
}

fn resolve_lipsync_mode(
    lipsync_expr: Option<&str>,
    fallback: LipSyncMode,
    input: &Value,
    metadata: &Value,
    language: &dyn crate::language::LanguageEngine,
) -> Result<LipSyncMode, PipelineError> {
    let Some(expr) = lipsync_expr.map(str::trim).filter(|expr| !expr.is_empty()) else {
        return Ok(fallback);
    };
    let value = eval_deno_expr(language, expr, input, metadata)?;
    match value {
        Value::String(raw) => parse_lipsync_mode(&raw),
        Value::Null => Ok(fallback),
        _ => Err(PipelineError::new(
            "AI_TTS_LIPSYNC",
            "lipsync_expr must resolve to a string or null",
        )),
    }
}

fn build_lipsync_payload(
    mode: LipSyncMode,
    text: &str,
    duration_ms: u64,
    sample_rate: i64,
    wav_bytes: &[u8],
) -> Result<(Value, Value), PipelineError> {
    if mode == LipSyncMode::None {
        return Ok((Value::Null, Value::Null));
    }
    let words = tokenize_words(text);
    if words.is_empty() || duration_ms == 0 {
        return Ok((
            Value::Array(Vec::new()),
            json!({
                "metadata": {
                    "duration": duration_ms,
                    "language": "id",
                    "phoneme_method": mode.as_str(),
                },
                "cues": []
            }),
        ));
    }
    let timings = match mode {
        LipSyncMode::None => Vec::new(),
        LipSyncMode::Basic => basic_word_timings(&words, duration_ms),
        LipSyncMode::TimedWords => weighted_word_timings(&words, duration_ms),
        LipSyncMode::AudioGuided => {
            audio_guided_word_timings(&words, duration_ms, sample_rate, wav_bytes)
                .unwrap_or_else(|| weighted_word_timings(&words, duration_ms))
        }
        LipSyncMode::AudioSegmented => {
            audio_segmented_word_timings(&words, duration_ms, sample_rate, wav_bytes)
                .unwrap_or_else(|| {
                    audio_guided_word_timings(&words, duration_ms, sample_rate, wav_bytes)
                        .unwrap_or_else(|| weighted_word_timings(&words, duration_ms))
                })
        }
    };
    let word_timings = Value::Array(
        timings
            .iter()
            .map(|timing| {
                json!({
                    "word": timing.word,
                    "start_ms": timing.start_ms,
                    "end_ms": timing.end_ms,
                })
            })
            .collect(),
    );
    let cues = Value::Array(build_lipsync_cues(mode, &timings));
    Ok((
        word_timings,
        json!({
            "metadata": {
                "duration": duration_ms,
                "language": "id",
                "phoneme_method": mode.as_str(),
            },
            "cues": cues
        }),
    ))
}

fn build_lipsync_cues(mode: LipSyncMode, timings: &[WordTiming]) -> Vec<Value> {
    match mode {
        LipSyncMode::None => Vec::new(),
        LipSyncMode::Basic | LipSyncMode::TimedWords => timings
            .iter()
            .map(|timing| {
                json!({
                    "startTime": timing.start_ms,
                    "endTime": timing.end_ms,
                    "viseme": dominant_viseme(&timing.word),
                    "intensity": 1.0,
                    "phoneme": timing.word,
                })
            })
            .collect(),
        LipSyncMode::AudioGuided => timings
            .iter()
            .flat_map(expand_audio_guided_cues_for_word)
            .collect(),
        LipSyncMode::AudioSegmented => build_audio_segmented_cues(timings),
    }
}

#[derive(Debug, Clone)]
struct WordTiming {
    word: String,
    start_ms: u64,
    end_ms: u64,
}

#[derive(Debug, Clone)]
struct VisemeSegment {
    token: String,
    viseme: &'static str,
    weight: f32,
}

fn tokenize_words(text: &str) -> Vec<TimedWord> {
    let mut out = Vec::new();
    for raw in text.split_whitespace() {
        let word = raw
            .trim_matches(|ch: char| !ch.is_alphanumeric() && ch != '\'')
            .to_lowercase();
        if word.is_empty() {
            continue;
        }
        let pause_after_ms = if raw.ends_with("...") {
            300
        } else if raw.ends_with('.') || raw.ends_with('!') || raw.ends_with('?') {
            220
        } else if raw.ends_with(',') || raw.ends_with(';') || raw.ends_with(':') {
            120
        } else {
            0
        };
        out.push(TimedWord {
            weight: word_weight(&word),
            word,
            pause_after_ms,
        });
    }
    out
}

fn basic_word_timings(words: &[TimedWord], duration_ms: u64) -> Vec<WordTiming> {
    let count = words.len() as u64;
    let mut out = Vec::with_capacity(words.len());
    let mut current = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let start = current;
        let end = if index + 1 == words.len() {
            duration_ms
        } else {
            duration_ms.saturating_mul((index as u64) + 1) / count
        };
        current = end;
        out.push(WordTiming {
            word: word.word.clone(),
            start_ms: start,
            end_ms: end.max(start + 1),
        });
    }
    out
}

fn weighted_word_timings(words: &[TimedWord], duration_ms: u64) -> Vec<WordTiming> {
    if words.is_empty() {
        return Vec::new();
    }
    let total_pause_ms: u64 = words.iter().map(|word| word.pause_after_ms).sum();
    let spoken_ms = duration_ms.saturating_sub(total_pause_ms.min(duration_ms / 3));
    let total_weight: f32 = words.iter().map(|word| word.weight.max(0.1)).sum();
    if total_weight <= f32::EPSILON {
        return basic_word_timings(words, duration_ms);
    }
    let mut out = Vec::with_capacity(words.len());
    let mut spoken_cursor = 0_f32;
    let mut actual_cursor = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let start = actual_cursor;
        spoken_cursor += word.weight.max(0.1);
        let mut end = if index + 1 == words.len() {
            duration_ms
        } else {
            (((spoken_cursor / total_weight) * (spoken_ms as f32)).round() as u64)
                .saturating_add(actual_cursor.saturating_sub(start))
        };
        end = end.max(start + 1);
        actual_cursor = end.saturating_add(word.pause_after_ms);
        out.push(WordTiming {
            word: word.word.clone(),
            start_ms: start,
            end_ms: end.min(duration_ms),
        });
    }
    if let Some(last) = out.last_mut() {
        last.end_ms = duration_ms.max(last.start_ms + 1);
    }
    out
}

fn audio_guided_word_timings(
    words: &[TimedWord],
    duration_ms: u64,
    sample_rate: i64,
    wav_bytes: &[u8],
) -> Option<Vec<WordTiming>> {
    let analysis = analyze_wav_frames(sample_rate, wav_bytes, 0.18, 0.015)?;
    let voiced_weights: Vec<f32> = analysis
        .energies
        .iter()
        .map(|energy| {
            if *energy <= analysis.threshold {
                0.0
            } else {
                ((*energy - analysis.threshold) / (analysis.max_energy - analysis.threshold + 1e-6))
                    .max(0.1)
            }
        })
        .collect();
    let total_voiced: f32 = voiced_weights.iter().sum();
    if total_voiced <= 0.1 {
        return None;
    }
    let word_weights: Vec<f32> = words.iter().map(|word| word.weight.max(0.1)).collect();
    let total_word_weight: f32 = word_weights.iter().sum();
    if total_word_weight <= f32::EPSILON {
        return None;
    }
    let mut frame_cumulative = Vec::with_capacity(voiced_weights.len());
    let mut acc = 0.0_f32;
    for weight in &voiced_weights {
        acc += *weight;
        frame_cumulative.push(acc);
    }
    let map_target_to_ms = |target: f32| -> u64 {
        if target <= 0.0 {
            return 0;
        }
        for (frame_idx, cumulative) in frame_cumulative.iter().enumerate() {
            if *cumulative >= target {
                let prev = if frame_idx == 0 {
                    0.0
                } else {
                    frame_cumulative[frame_idx - 1]
                };
                let local_ratio = if (*cumulative - prev).abs() < f32::EPSILON {
                    0.0
                } else {
                    ((target - prev) / (*cumulative - prev)).clamp(0.0, 1.0)
                };
                let frame_start_ms = (frame_idx as u64) * analysis.frame_ms;
                let frame_end_ms = (((frame_idx as u64) + 1) * analysis.frame_ms).min(duration_ms);
                return frame_start_ms
                    + (((frame_end_ms - frame_start_ms) as f32) * local_ratio).round() as u64;
            }
        }
        duration_ms
    };
    let mut out = Vec::with_capacity(words.len());
    let mut cumulative_word = 0.0_f32;
    let mut prev_end = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let start = if index == 0 {
            map_target_to_ms(0.0)
        } else {
            prev_end
        };
        cumulative_word += word_weights[index];
        let mut end = if index + 1 == words.len() {
            duration_ms
        } else {
            map_target_to_ms((cumulative_word / total_word_weight) * total_voiced)
        };
        end = end.max(start + 1).min(duration_ms);
        prev_end = end;
        out.push(WordTiming {
            word: word.word.clone(),
            start_ms: start,
            end_ms: end,
        });
    }
    Some(out)
}

fn audio_segmented_word_timings(
    words: &[TimedWord],
    duration_ms: u64,
    sample_rate: i64,
    wav_bytes: &[u8],
) -> Option<Vec<WordTiming>> {
    let analysis = analyze_wav_frames(sample_rate, wav_bytes, 0.26, 0.02)?;
    let base = audio_guided_word_timings(words, duration_ms, sample_rate, wav_bytes)?;
    let mut refined = Vec::with_capacity(base.len());
    for timing in base {
        let mut start_frame =
            ms_to_frame_idx(timing.start_ms, analysis.frame_ms, analysis.energies.len());
        let mut end_frame =
            ms_to_frame_idx_ceil(timing.end_ms, analysis.frame_ms, analysis.energies.len());
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(analysis.energies.len());
        }
        while start_frame < end_frame && analysis.energies[start_frame] <= analysis.threshold {
            start_frame += 1;
        }
        while end_frame > start_frame && analysis.energies[end_frame - 1] <= analysis.threshold {
            end_frame -= 1;
        }
        let refined_start = (start_frame as u64 * analysis.frame_ms).min(duration_ms);
        let refined_end =
            (((end_frame as u64) * analysis.frame_ms).min(duration_ms)).max(refined_start + 1);
        refined.push(WordTiming {
            word: timing.word,
            start_ms: refined_start,
            end_ms: refined_end.min(duration_ms),
        });
    }
    reconcile_word_boundaries(&mut refined, duration_ms);
    Some(refined)
}

fn reconcile_word_boundaries(timings: &mut [WordTiming], duration_ms: u64) {
    if timings.is_empty() {
        return;
    }
    for index in 1..timings.len() {
        let prev_end = timings[index - 1].end_ms;
        let curr_start = timings[index].start_ms;
        if curr_start < prev_end {
            let midpoint = ((curr_start + prev_end) / 2).max(timings[index - 1].start_ms + 1);
            timings[index - 1].end_ms = midpoint.min(duration_ms);
            timings[index].start_ms = midpoint.min(timings[index].end_ms.saturating_sub(1));
        }
    }
    for timing in timings.iter_mut() {
        timing.end_ms = timing.end_ms.max(timing.start_ms + 1).min(duration_ms);
    }
}

fn build_audio_segmented_cues(timings: &[WordTiming]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut previous_end: Option<u64> = None;
    for timing in timings {
        if let Some(prev_end) = previous_end.filter(|prev_end| timing.start_ms > *prev_end) {
            out.push(json!({
                "startTime": prev_end,
                "endTime": timing.start_ms,
                "viseme": "neutral",
                "intensity": 0.35,
                "phoneme": "gap",
                "word": Value::Null,
            }));
        }
        out.extend(expand_audio_guided_cues_for_word(timing));
        previous_end = Some(timing.end_ms);
    }
    out
}

#[derive(Debug, Clone)]
struct WavFrameAnalysis {
    frame_ms: u64,
    energies: Vec<f32>,
    max_energy: f32,
    threshold: f32,
}

fn analyze_wav_frames(
    sample_rate: i64,
    wav_bytes: &[u8],
    threshold_ratio: f32,
    min_threshold: f32,
) -> Option<WavFrameAnalysis> {
    let sample_rate = u32::try_from(sample_rate).ok()?;
    let reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes)).ok()?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    let samples: Vec<f32> = reader
        .into_samples::<i16>()
        .filter_map(Result::ok)
        .map(|sample| sample as f32 / i16::MAX as f32)
        .collect();
    if samples.is_empty() {
        return None;
    }
    let mono: Vec<f32> = if channels == 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|chunk| chunk.iter().copied().sum::<f32>() / channels as f32)
            .collect()
    };
    let frame_ms = 20_u64;
    let frame_len = ((sample_rate as u64 * frame_ms) / 1000).max(1) as usize;
    let mut energies = Vec::new();
    let mut index = 0usize;
    while index < mono.len() {
        let end = (index + frame_len).min(mono.len());
        let slice = &mono[index..end];
        let rms =
            (slice.iter().map(|sample| sample * sample).sum::<f32>() / slice.len() as f32).sqrt();
        energies.push(rms);
        index = end;
    }
    let max_energy = energies.iter().copied().fold(0.0_f32, f32::max);
    if max_energy <= 0.0001 {
        return None;
    }
    let threshold = (max_energy * threshold_ratio).max(min_threshold);
    Some(WavFrameAnalysis {
        frame_ms,
        energies,
        max_energy,
        threshold,
    })
}

fn ms_to_frame_idx(ms: u64, frame_ms: u64, frame_count: usize) -> usize {
    ((ms / frame_ms) as usize).min(frame_count.saturating_sub(1))
}

fn ms_to_frame_idx_ceil(ms: u64, frame_ms: u64, frame_count: usize) -> usize {
    let ceil = ms.div_ceil(frame_ms) as usize;
    ceil.min(frame_count)
}

fn expand_audio_guided_cues_for_word(timing: &WordTiming) -> Vec<Value> {
    let segments = split_word_into_viseme_segments(&timing.word);
    if segments.is_empty() || timing.end_ms <= timing.start_ms {
        return vec![json!({
            "startTime": timing.start_ms,
            "endTime": timing.end_ms.max(timing.start_ms + 1),
            "viseme": dominant_viseme(&timing.word),
            "intensity": 1.0,
            "phoneme": timing.word,
            "word": timing.word,
        })];
    }
    let total_weight: f32 = segments.iter().map(|segment| segment.weight.max(0.1)).sum();
    let total_duration = timing.end_ms.saturating_sub(timing.start_ms);
    let mut cumulative = 0.0_f32;
    let mut current_start = timing.start_ms;
    let mut out = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        cumulative += segment.weight.max(0.1);
        let mut next_end = if index + 1 == segments.len() {
            timing.end_ms
        } else {
            timing.start_ms
                + (((cumulative / total_weight) * (total_duration as f32)).round() as u64)
        };
        next_end = next_end.max(current_start + 1).min(timing.end_ms);
        out.push(json!({
            "startTime": current_start,
            "endTime": next_end,
            "viseme": segment.viseme,
            "intensity": 1.0,
            "phoneme": segment.token,
            "word": timing.word,
        }));
        current_start = next_end;
    }
    out
}

fn split_word_into_viseme_segments(word: &str) -> Vec<VisemeSegment> {
    let mut out = Vec::new();
    let mut consonant_run = String::new();
    let chars: Vec<char> = word.chars().collect();
    for (index, ch) in chars.iter().copied().enumerate() {
        if is_vowel(ch) {
            if !consonant_run.is_empty() {
                out.extend(split_consonant_run(&consonant_run));
                consonant_run.clear();
            }
            out.push(VisemeSegment {
                token: ch.to_string(),
                viseme: vowel_viseme_in_word(&chars, index),
                weight: 1.65,
            });
        } else if ch.is_alphabetic() {
            consonant_run.push(ch);
        }
    }
    if !consonant_run.is_empty() {
        out.extend(split_consonant_run(&consonant_run));
    }
    if out.is_empty() {
        out.push(VisemeSegment {
            token: word.to_string(),
            viseme: dominant_viseme(word),
            weight: 1.0,
        });
    }
    out
}

fn split_consonant_run(run: &str) -> Vec<VisemeSegment> {
    let mut out = Vec::new();
    let mut neutral_buf = String::new();
    let mut close_buf = String::new();
    let flush_neutral = |buf: &mut String, out: &mut Vec<VisemeSegment>| {
        if !buf.is_empty() {
            out.push(VisemeSegment {
                token: std::mem::take(buf),
                viseme: "neutral",
                weight: 0.7,
            });
        }
    };
    let flush_close = |buf: &mut String, out: &mut Vec<VisemeSegment>| {
        if !buf.is_empty() {
            out.push(VisemeSegment {
                token: std::mem::take(buf),
                viseme: "close",
                weight: 0.9,
            });
        }
    };
    let chars: Vec<char> = run.chars().collect();
    let mut index = 0usize;
    while index < chars.len() {
        let token = if index + 1 < chars.len() {
            match (chars[index], chars[index + 1]) {
                ('n', 'g') | ('n', 'y') | ('s', 'y') | ('k', 'h') => {
                    index += 2;
                    format!("{}{}", chars[index - 2], chars[index - 1])
                }
                _ => {
                    index += 1;
                    chars[index - 1].to_string()
                }
            }
        } else {
            index += 1;
            chars[index - 1].to_string()
        };
        if token.chars().all(is_close_consonant) {
            flush_neutral(&mut neutral_buf, &mut out);
            close_buf.push_str(&token);
        } else {
            flush_close(&mut close_buf, &mut out);
            neutral_buf.push_str(&token);
        }
    }
    flush_neutral(&mut neutral_buf, &mut out);
    flush_close(&mut close_buf, &mut out);
    out
}

fn is_vowel(ch: char) -> bool {
    matches!(ch, 'a' | 'i' | 'u' | 'e' | 'o')
}

fn is_close_consonant(ch: char) -> bool {
    matches!(ch, 'p' | 'b' | 'm')
}

fn vowel_viseme_in_word(chars: &[char], index: usize) -> &'static str {
    let ch = chars[index];
    match ch {
        'a' => "aa",
        'i' => "ih",
        'u' => "ou",
        'e' => {
            if looks_like_schwa(chars, index) {
                "eh"
            } else {
                "ee"
            }
        }
        'o' => "oh",
        _ => "neutral",
    }
}

fn looks_like_schwa(chars: &[char], index: usize) -> bool {
    let prev = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
    let next = chars.get(index + 1).copied();
    let next2 = chars.get(index + 2).copied();
    match (prev, next, next2) {
        (Some(p), Some(n), Some(n2)) if !is_vowel(p) && !is_vowel(n) && is_vowel(n2) => true,
        (None, Some('m' | 'n' | 'r' | 'l' | 's' | 't' | 'k' | 'p' | 'b'), Some(n2))
            if !is_vowel(n2) =>
        {
            true
        }
        (Some(p), Some(n), None) if !is_vowel(p) && !is_vowel(n) => true,
        _ => false,
    }
}

fn word_weight(word: &str) -> f32 {
    let chars = word.chars().count() as f32;
    let vowels = word
        .chars()
        .filter(|ch| matches!(ch, 'a' | 'i' | 'u' | 'e' | 'o'))
        .count() as f32;
    (chars * 0.7) + (vowels * 1.3)
}

fn dominant_viseme(word: &str) -> &'static str {
    match word {
        "terima" | "kasih" | "apa" | "saya" | "akan" | "datang" | "pagi" | "malam" | "jalan"
        | "makan" | "aman" | "karena" | "bahasa" | "sekarang" | "tentang" => "aa",
        "ini" | "dingin" | "kiri" | "minim" | "pilih" | "ingin" | "bisa" | "kecil" | "sedikit"
        | "istri" | "lihat" | "hidup" => "ih",
        "buku" | "untuk" | "umur" | "turun" | "gunung" | "cukup" | "musik" | "murung" | "suruh"
        | "rumput" => "ou",
        "enak" | "meja" | "lebar" | "besok" | "cepat" | "kereta" | "teman" | "seret" | "dekat"
        | "hemat" => "ee",
        "orang" | "tolong" | "mobil" | "sore" | "kota" | "dokter" | "nomor" | "kosong"
        | "obrolan" | "boleh" => "oh",
        _ => fallback_viseme(word),
    }
}

fn fallback_viseme(word: &str) -> &'static str {
    let mut counts = [0_u32; 5];
    for ch in word.chars() {
        match ch {
            'a' => counts[0] += 1,
            'i' => counts[1] += 1,
            'u' => counts[2] += 1,
            'e' => counts[3] += 1,
            'o' => counts[4] += 1,
            _ => {}
        }
    }
    let (index, max_count) = counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .unwrap_or((0, 0));
    if max_count == 0 {
        return "aa";
    }
    match index {
        0 => "aa",
        1 => "ih",
        2 => "ou",
        3 => "ee",
        4 => "oh",
        _ => "aa",
    }
}

fn resolve_output_rel_path(
    output_path: Option<&str>,
    output_path_expr: Option<&str>,
    input: &Value,
    metadata: &Value,
    language: &dyn crate::language::LanguageEngine,
) -> Result<String, PipelineError> {
    if let Some(expr) = output_path_expr
        .map(str::trim)
        .filter(|expr| !expr.is_empty())
    {
        let value = eval_deno_expr(language, expr, input, metadata)?;
        return value_as_non_empty_string("output_path_expr", &value);
    }
    let Some(path) = output_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Err(PipelineError::new(
            "AI_TTS_OUTPUT_PATH",
            "output_path or output_path_expr is required when return mode writes a file",
        ));
    };
    Ok(path.to_string())
}

fn resolve_zebfs_abs(files_root: &Path, raw: &str) -> Result<PathBuf, PipelineError> {
    Ok(files_root.join(normalize_zebfs_asset_rel_path(raw)?))
}

fn normalize_zebfs_asset_rel_path(raw: &str) -> Result<String, PipelineError> {
    let normalized = raw.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err(PipelineError::new("AI_TTS_PATH", "path must not be empty"));
    }
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "AI_TTS_PATH",
                "path must stay inside project Zebflow FS",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "AI_TTS_PATH",
            "path must not resolve to the Zebflow FS root itself",
        ));
    }
    Ok(parts.join("/"))
}

fn normalize_audio_output_rel_path(raw: &str) -> Result<String, PipelineError> {
    let mut rel = normalize_zebfs_asset_rel_path(raw)?;
    let ext = Path::new(&rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if !ext.is_empty() && ext != "wav" {
        return Err(PipelineError::new(
            "AI_TTS_OUTPUT_PATH",
            "output path must use .wav extension when an extension is provided",
        ));
    }
    if ext.is_empty() && !rel.ends_with('/') {
        rel.push_str(".wav");
    }
    Ok(rel)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Instant;

    use serde_json::json;

    use super::{
        Config, LipSyncMode, Node, ReturnMode, WordTiming, basic_word_timings,
        build_audio_segmented_cues, expand_audio_guided_cues_for_word,
        normalize_audio_output_rel_path, parse_lipsync_mode, reconcile_word_boundaries,
        speed_to_length_scale, split_word_into_viseme_segments, tokenize_words,
        weighted_word_timings,
    };
    use crate::language::DenoSandboxEngine;
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::{PlatformConfig, UpsertProjectCredentialRequest};
    use crate::platform::services::PlatformService;

    fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let target = dst.join(entry.file_name());
            if ty.is_dir() {
                copy_dir_all(&entry.path(), &target)?;
            } else {
                fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }

    #[test]
    fn audio_output_path_normalization_adds_wav() {
        assert_eq!(
            normalize_audio_output_rel_path("audio/demo").expect("normalize"),
            "audio/demo.wav"
        );
        assert_eq!(
            normalize_audio_output_rel_path("audio/demo.wav").expect("normalize"),
            "audio/demo.wav"
        );
    }

    #[test]
    fn audio_output_path_normalization_rejects_escape_and_non_wav() {
        assert!(normalize_audio_output_rel_path("../audio/demo.wav").is_err());
        assert!(normalize_audio_output_rel_path("audio/demo.mp3").is_err());
    }

    #[test]
    fn speed_maps_to_length_scale() {
        assert_eq!(speed_to_length_scale(1.0).expect("scale"), None);
        let value = speed_to_length_scale(2.0)
            .expect("scale")
            .expect("length scale");
        assert!((value - 0.5).abs() < f32::EPSILON);
        assert!(speed_to_length_scale(0.0).is_err());
    }

    #[test]
    fn parse_lipsync_modes_accepts_aliases() {
        assert_eq!(parse_lipsync_mode("none").expect("mode"), LipSyncMode::None);
        assert_eq!(
            parse_lipsync_mode("word_to_vowel").expect("mode"),
            LipSyncMode::Basic
        );
        assert_eq!(
            parse_lipsync_mode("timed").expect("mode"),
            LipSyncMode::TimedWords
        );
        assert_eq!(
            parse_lipsync_mode("audio").expect("mode"),
            LipSyncMode::AudioGuided
        );
        assert_eq!(
            parse_lipsync_mode("segmented").expect("mode"),
            LipSyncMode::AudioSegmented
        );
        assert!(parse_lipsync_mode("nope").is_err());
    }

    #[test]
    fn timing_strategies_return_ordered_ranges() {
        let words = tokenize_words("Halo dunia, ini contoh bagus.");
        let basic = basic_word_timings(&words, 2000);
        let timed = weighted_word_timings(&words, 2000);
        assert_eq!(basic.len(), 5);
        assert_eq!(timed.len(), 5);
        assert_eq!(basic.first().expect("first").start_ms, 0);
        assert_eq!(timed.first().expect("first").start_ms, 0);
        assert_eq!(basic.last().expect("last").end_ms, 2000);
        assert_eq!(timed.last().expect("last").end_ms, 2000);
        assert!(
            timed[1].end_ms > basic[1].end_ms || timed[2].start_ms > basic[2].start_ms,
            "timed_words should shift timing away from pure even slicing"
        );
    }

    #[test]
    fn audio_guided_segments_split_words_into_sub_visemes() {
        let segments = split_word_into_viseme_segments("transportasi");
        let tokens: Vec<&str> = segments
            .iter()
            .map(|segment| segment.token.as_str())
            .collect();
        let visemes: Vec<&str> = segments.iter().map(|segment| segment.viseme).collect();
        assert_eq!(tokens, vec!["tr", "a", "ns", "p", "o", "rt", "a", "s", "i"]);
        assert_eq!(
            visemes,
            vec![
                "neutral", "aa", "neutral", "close", "oh", "neutral", "aa", "neutral", "ih"
            ]
        );

        let cues = expand_audio_guided_cues_for_word(&WordTiming {
            word: "kemarin".to_string(),
            start_ms: 1000,
            end_ms: 1800,
        });
        assert!(cues.len() >= 5);
        assert_eq!(cues.first().expect("first")["startTime"], json!(1000));
        assert_eq!(cues.last().expect("last")["endTime"], json!(1800));
    }

    #[test]
    fn indonesian_clusters_and_schwa_are_detected() {
        let banyak = split_word_into_viseme_segments("banyak");
        let tokens: Vec<&str> = banyak
            .iter()
            .map(|segment| segment.token.as_str())
            .collect();
        let visemes: Vec<&str> = banyak.iter().map(|segment| segment.viseme).collect();
        assert_eq!(tokens, vec!["b", "a", "ny", "a", "k"]);
        assert_eq!(visemes, vec!["close", "aa", "neutral", "aa", "neutral"]);

        let kemarin = split_word_into_viseme_segments("kemarin");
        assert_eq!(kemarin[1].token, "e");
        assert_eq!(kemarin[1].viseme, "eh");
    }

    #[test]
    fn audio_segmented_inserts_neutral_gap_cues() {
        let cues = build_audio_segmented_cues(&[
            WordTiming {
                word: "halo".to_string(),
                start_ms: 100,
                end_ms: 220,
            },
            WordTiming {
                word: "ini".to_string(),
                start_ms: 280,
                end_ms: 420,
            },
        ]);
        assert_eq!(cues[0]["startTime"], json!(100));
        assert!(cues.iter().any(|cue| cue["phoneme"] == json!("gap")));
        let gap = cues
            .iter()
            .find(|cue| cue["phoneme"] == json!("gap"))
            .expect("gap cue");
        assert_eq!(gap["startTime"], json!(220));
        assert_eq!(gap["endTime"], json!(280));
        assert_eq!(gap["viseme"], json!("neutral"));
    }

    #[test]
    fn reconcile_word_boundaries_removes_overlap() {
        let mut timings = vec![
            WordTiming {
                word: "halo".to_string(),
                start_ms: 140,
                end_ms: 320,
            },
            WordTiming {
                word: "ini".to_string(),
                start_ms: 300,
                end_ms: 480,
            },
        ];
        reconcile_word_boundaries(&mut timings, 1000);
        assert!(timings[0].end_ms <= timings[1].start_ms);
        assert_eq!(timings[0].end_ms, 310);
        assert_eq!(timings[1].start_ms, 310);
    }

    #[tokio::test]
    #[ignore = "manual local smoke using Arin Piper assets"]
    async fn piper_node_smoke_with_local_arin() {
        let model_src = Path::new("/Users/mala0061/Dev/open-obe/avatar/arin-2449.onnx");
        let config_src = Path::new("/Users/mala0061/Dev/open-obe/avatar/arin-2449.onnx.json");
        let espeak_src = Path::new(
            "/Users/mala0061/Dev/tools/assistant/angelta-web/fastapi-backend/tts/sherpaonnx/espeak-ng-data",
        );
        if !(model_src.is_file() && config_src.is_file() && espeak_src.is_dir()) {
            eprintln!("local Arin smoke assets are not present; skipping");
            return;
        }

        let data_root = std::env::temp_dir().join("zf-tts-node-smoke-test");
        let _ = fs::remove_dir_all(&data_root);
        let mut cfg = PlatformConfig::default();
        cfg.data_root = data_root.clone();
        cfg.default_password = "secret".to_string();
        let platform = Arc::new(PlatformService::from_config(cfg).expect("platform"));
        let layout = platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");

        let voice_dir = layout.files_dir.join("voices/arin");
        fs::create_dir_all(&voice_dir).expect("voice dir");
        fs::copy(model_src, voice_dir.join("arin-2449.onnx")).expect("copy model");
        fs::copy(config_src, voice_dir.join("arin-2449.onnx.json")).expect("copy config");
        copy_dir_all(espeak_src, &layout.files_dir.join("runtime/espeak-ng-data"))
            .expect("copy espeak");

        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "arin-tts".to_string(),
                    title: "Arin TTS".to_string(),
                    kind: "tts".to_string(),
                    secret: json!({
                        "provider": "piper",
                        "model_file": "voices/arin/arin-2449.onnx",
                        "config_file": "voices/arin/arin-2449.onnx.json",
                        "espeak_data_dir": "runtime/espeak-ng-data",
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let node = Node::new(
            Config {
                provider: "piper".to_string(),
                credential_id: "arin-tts".to_string(),
                text_expr: "'Halo, ini Arin dari node n.ai.tts Zebflow.'".to_string(),
                output_path: Some("audio/arin-node-smoke.wav".to_string()),
                output_path_expr: None,
                return_mode: ReturnMode::Both,
                speaker: None,
                speed: 1.0,
                volume: 1.0,
                lipsync_mode: LipSyncMode::Basic,
                lipsync_expr: None,
            },
            Some(platform.credentials.clone()),
            Some(platform.clone()),
            Arc::new(DenoSandboxEngine::default()),
        );

        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "tts".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "tts-smoke",
                    "request_id": "req-1",
                    "trigger": { "kind": "manual" }
                }),
                step_tx: None,
            })
            .await
            .expect("execute");

        let file_rel = out.payload["audio"]["path"]
            .as_str()
            .expect("audio path should be present");
        assert_eq!(file_rel, "audio/arin-node-smoke.wav");
        assert!(
            layout.files_dir.join(file_rel).is_file(),
            "expected synthesized wav file to exist"
        );
        assert!(
            out.payload["audio_blob_base64"]
                .as_str()
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            "expected inline audio blob"
        );
        assert_eq!(
            out.payload["lipsync"]["metadata"]["phoneme_method"],
            json!("basic")
        );
        assert!(
            out.payload["word_timings"]
                .as_array()
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    #[ignore = "manual local benchmark using Arin Piper assets"]
    async fn piper_lipsync_benchmark_with_local_arin() {
        let model_src = Path::new("/Users/mala0061/Dev/open-obe/avatar/arin-2449.onnx");
        let config_src = Path::new("/Users/mala0061/Dev/open-obe/avatar/arin-2449.onnx.json");
        let espeak_src = Path::new(
            "/Users/mala0061/Dev/tools/assistant/angelta-web/fastapi-backend/tts/sherpaonnx/espeak-ng-data",
        );
        if !(model_src.is_file() && config_src.is_file() && espeak_src.is_dir()) {
            eprintln!("local Arin benchmark assets are not present; skipping");
            return;
        }

        let data_root = std::env::temp_dir().join("zf-tts-node-benchmark");
        let _ = fs::remove_dir_all(&data_root);
        let mut cfg = PlatformConfig::default();
        cfg.data_root = data_root.clone();
        cfg.default_password = "secret".to_string();
        let platform = Arc::new(PlatformService::from_config(cfg).expect("platform"));
        let layout = platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");

        let voice_dir = layout.files_dir.join("voices/arin");
        fs::create_dir_all(&voice_dir).expect("voice dir");
        fs::copy(model_src, voice_dir.join("arin-2449.onnx")).expect("copy model");
        fs::copy(config_src, voice_dir.join("arin-2449.onnx.json")).expect("copy config");
        copy_dir_all(espeak_src, &layout.files_dir.join("runtime/espeak-ng-data"))
            .expect("copy espeak");

        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "arin-tts".to_string(),
                    title: "Arin TTS".to_string(),
                    kind: "tts".to_string(),
                    secret: json!({
                        "provider": "piper",
                        "model_file": "voices/arin/arin-2449.onnx",
                        "config_file": "voices/arin/arin-2449.onnx.json",
                        "espeak_data_dir": "runtime/espeak-ng-data",
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let sentences = vec![
            "Halo, saya sedang mencoba suara Arin untuk demo Zebflow hari ini.",
            "Tolong kirim ringkasan rapat jam tiga sore ke semua anggota tim.",
            "Cuaca di Bandung mendung sejak pagi, tetapi jalanan masih cukup ramai.",
            "Kalau koneksi internet putus sebentar, sistem harus bisa pulih tanpa panik.",
            "Mari kita bandingkan metode lipsync sederhana dengan pendekatan audio yang lebih peka.",
        ];
        let modes = [
            LipSyncMode::None,
            LipSyncMode::Basic,
            LipSyncMode::TimedWords,
            LipSyncMode::AudioGuided,
        ];

        let warmup = Node::new(
            Config {
                provider: "piper".to_string(),
                credential_id: "arin-tts".to_string(),
                text_expr: "'Warmup untuk benchmark lipsync Zebflow.'".to_string(),
                output_path: None,
                output_path_expr: None,
                return_mode: ReturnMode::Blob,
                speaker: None,
                speed: 1.0,
                volume: 1.0,
                lipsync_mode: LipSyncMode::None,
                lipsync_expr: None,
            },
            Some(platform.credentials.clone()),
            Some(platform.clone()),
            Arc::new(DenoSandboxEngine::default()),
        );
        let _ = warmup
            .execute_async(NodeExecutionInput {
                node_id: "tts".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "tts-bench-warmup",
                    "request_id": "bench-warmup",
                    "trigger": { "kind": "manual" }
                }),
                step_tx: None,
            })
            .await
            .expect("warmup");

        eprintln!("| Sentence | none | basic | timed_words | audio_guided |");
        eprintln!("|---|---:|---:|---:|---:|");
        for sentence in sentences {
            let mut row = vec![sentence.to_string()];
            for mode in modes {
                let node = Node::new(
                    Config {
                        provider: "piper".to_string(),
                        credential_id: "arin-tts".to_string(),
                        text_expr: format!("{sentence:?}"),
                        output_path: None,
                        output_path_expr: None,
                        return_mode: ReturnMode::Blob,
                        speaker: None,
                        speed: 1.0,
                        volume: 1.0,
                        lipsync_mode: mode,
                        lipsync_expr: None,
                    },
                    Some(platform.credentials.clone()),
                    Some(platform.clone()),
                    Arc::new(DenoSandboxEngine::default()),
                );

                let started = Instant::now();
                let out = node
                    .execute_async(NodeExecutionInput {
                        node_id: "tts".to_string(),
                        input_pin: "in".to_string(),
                        payload: json!({}),
                        metadata: json!({
                            "owner": "superadmin",
                            "project": "default",
                            "pipeline": "tts-bench",
                            "request_id": format!("bench-{}", mode.as_str()),
                            "trigger": { "kind": "manual" }
                        }),
                        step_tx: None,
                    })
                    .await
                    .expect("execute");
                let elapsed_ms = started.elapsed().as_millis();
                if mode != LipSyncMode::None {
                    assert!(out.payload["lipsync"].is_object());
                }
                row.push(format!("{elapsed_ms} ms"));
            }
            eprintln!(
                "| {} | {} | {} | {} | {} |",
                row[0], row[1], row[2], row[3], row[4]
            );
        }
    }
}
