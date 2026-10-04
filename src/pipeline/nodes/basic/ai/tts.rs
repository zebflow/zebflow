//! `ai.audio.generate` — speech from text, one kind for every TTS provider
//! (`node-conventions.md` §11).
//!
//! | Use | DSL |
//! |---|---|
//! | A file in the store | `\| ai.audio.generate --provider piper --credential narrator --text "{{ input.body }}" --filename welcome` |
//! | Bytes for a page to play | `\| ai.audio.generate --provider piper --credential narrator --text "Hello" --return inline` |
//! | Mouth cues for an avatar | `\| ai.audio.generate --provider piper --credential narrator --text "Hello" --option lipsync=timed_words` |
//!
//! Providers: `piper`, a local Python runtime whose voice files live in the
//! project store at the keys its `tts` credential names. `--voice` and
//! `--speed` are words every TTS takes; a provider's own settings are
//! `--option` keys its profile closes (`volume`, `lipsync` for piper).
//!
//! Answer: `audio: { …FileRef…, format, mime_type, provider, sample_rate,
//! samples, duration_ms, word_timings?, lipsync? }` with `--return file`
//! (the default), or the same with `base64` and `size` in place of the
//! FileRef with `--return inline`. The payload is kept.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::profile::{check_credential, check_profile, option_flag, option_text, provider_flag};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::metadata_scope;
use crate::pipeline::PipelineError;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeDefinition, NodeFieldDataSource, NodeFieldDef,
    NodeFieldType, ProfileOption, ProviderProfile, SelectOptionDef,
};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::platform::services::{CredentialService, PlatformService};

pub const NODE_KIND: &str = "ai.audio.generate";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

const CONFIG_CODE: &str = "FW_NODE_AI_AUDIO_GENERATE_CONFIG";
const CREDENTIAL_CODE: &str = "FW_NODE_AI_AUDIO_GENERATE_CREDENTIAL";
const FILE_CODE: &str = "FW_NODE_AI_AUDIO_GENERATE_FILE";

/// The TTS providers, each with a profile.
pub const PROVIDERS: &[&str] = &["piper"];
/// `--return`: the bytes in the answer, or a file in the store.
const RETURNS: &[&str] = &["inline", "file"];
/// `--option lipsync=…` for piper.
const LIPSYNC_MODES: &[&str] = &["none", "basic", "timed_words", "audio_guided", "audio_segmented"];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnMode {
    Inline,
    File,
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

/// The node's config as stored: every value a literal or a resolved
/// `{{ }}`. [`Node::build`] reads it into [`Settings`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// `piper`; checked against the profiles before this is read.
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub credential_id: String,
    /// What to say — a literal or `{{ expr }}`, arriving final.
    #[serde(default)]
    pub text: String,
    /// The voice: for piper, a speaker id of a multi-speaker model.
    #[serde(default)]
    pub voice: Value,
    /// Speed factor, 1 = normal; greater is faster.
    #[serde(default)]
    pub speed: Value,
    /// The provider's own settings, closed by its profile.
    #[serde(default)]
    pub option: std::collections::BTreeMap<String, Value>,
    /// `inline` or `file` (default).
    #[serde(default, rename = "return")]
    pub return_mode: String,
    /// Store folder for the audio file (default: `audio`).
    #[serde(default)]
    pub folder: String,
    /// Audio file name; `.wav` is added (default: a UUID).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

/// What a built node runs with: the config read and checked once.
#[derive(Debug, Clone)]
struct Settings {
    return_mode: ReturnMode,
    voice: Option<i32>,
    speed: f32,
    volume: f32,
    lipsync: LipSyncMode,
}

/// A number flag or option as the DSL or the editor sends it; unset is `None`.
fn number(value: &Value, what: &str) -> Result<Option<f64>, PipelineError> {
    let shown = match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let bad = || PipelineError::new(CONFIG_CODE, format!("{what} '{shown}' is not a number"));
    match value {
        Value::Null => Ok(None),
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|n| n.is_finite()).map(Some).ok_or_else(bad),
        Value::Number(n) => n.as_f64().map(Some).ok_or_else(bad),
        _ => Err(bad()),
    }
}

impl Config {
    fn settings(&self) -> Result<Settings, PipelineError> {
        let return_mode = match choice(&self.return_mode, RETURNS, "file", "--return", CONFIG_CODE)? {
            "inline" => ReturnMode::Inline,
            _ => ReturnMode::File,
        };
        let voice = match number(&self.voice, "--voice")? {
            None => None,
            Some(id) if id.fract() == 0.0 && id >= 0.0 && id <= f64::from(i32::MAX) => Some(id as i32),
            Some(id) => {
                return Err(PipelineError::new(CONFIG_CODE, format!("--voice {id}: a piper voice is a speaker id, a whole number")));
            }
        };
        let speed = number(&self.speed, "--speed")?.unwrap_or(1.0) as f32;
        speed_to_length_scale(speed)?;
        let option = Value::Object(self.option.clone().into_iter().collect());
        let volume = match option_text(&option, "volume") {
            Some(text) => number(&Value::String(text), "--option volume")?.unwrap_or(1.0) as f32,
            None => 1.0,
        };
        let lipsync = match choice(&option_text(&option, "lipsync").unwrap_or_default(), LIPSYNC_MODES, "none", "--option lipsync", CONFIG_CODE)? {
            "basic" => LipSyncMode::Basic,
            "timed_words" => LipSyncMode::TimedWords,
            "audio_guided" => LipSyncMode::AudioGuided,
            "audio_segmented" => LipSyncMode::AudioSegmented,
            _ => LipSyncMode::None,
        };
        Ok(Settings { return_mode, voice, speed, volume, lipsync })
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
    audio_blob_base64: String,
}

/// What a provider synthesised: wav bytes and their measure.
struct Synthesis {
    wav: Vec<u8>,
    sample_rate: i64,
    samples: usize,
    duration_ms: u64,
}

#[derive(Debug, Clone)]
struct TimedWord {
    word: String,
    weight: f32,
    pause_after_ms: u64,
}

fn profiles() -> Vec<ProviderProfile> {
    vec![ProviderProfile {
        provider: "piper".to_string(),
        credential_kinds: vec!["tts".to_string()],
        options: vec![
            ProfileOption {
                key: "volume".to_string(),
                value: "number".to_string(),
                description: "Loudness multiplier; 1 is as the voice was recorded.".to_string(),
                ..Default::default()
            },
            ProfileOption {
                key: "lipsync".to_string(),
                value: "text".to_string(),
                choices: LIPSYNC_MODES.iter().map(|m| m.to_string()).collect(),
                description: "Mouth cues and word timings in the answer: none (default), basic, timed_words, audio_guided or audio_segmented.".to_string(),
            },
        ],
        ..Default::default()
    }]
}

/// The definition, built once: every build of the node checks its config
/// against the profiles here.
fn checked_definition() -> &'static NodeDefinition {
    static DEFINITION: std::sync::LazyLock<NodeDefinition> = std::sync::LazyLock::new(definition);
    &DEFINITION
}

/// The config against the chosen provider's profile.
pub fn check_config_profile(config: &Value) -> Result<(), PipelineError> {
    check_profile(checked_definition(), config).map_err(|message| PipelineError::new(CONFIG_CODE, message))
}

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

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|w| w.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Credential, NodeCapability::Process],
        title: "AI Audio".to_string(),
        description: "Speech from text. `--provider piper` runs a local Piper voice named by a `tts` credential (the owner \
            puts the voice files in the store; the credential names their keys). `--text` is what to say; `--voice` and \
            `--speed` shape it; the provider's own settings are `--option` keys (piper: volume, lipsync). `--return file` \
            (default) writes a wav at `--path`, or `--folder`/`--filename` (default folder `audio`), and answers \
            `audio: { …FileRef…, format, mime_type, provider, sample_rate, samples, duration_ms }`; `--return inline` writes \
            nothing and answers the bytes as `audio.base64`, which a page plays at once. Keeps the payload."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "The payload the {{ expr }} in --text resolve against. The node reads nothing else of it."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "audio": {
                    "type": "object",
                    "description": "With --return file: the written wav's FileRef fields (ref, store, filename, mime, kind, size, sha256, …) and the rest. With --return inline: base64 and size in their place.",
                    "properties": {
                        "provider": { "type": "string" },
                        "format": { "type": "string" },
                        "mime_type": { "type": "string" },
                        "ref": { "type": "string", "description": "--return file: the store key written." },
                        "base64": { "type": "string", "description": "--return inline: the wav bytes." },
                        "size": { "type": "integer" },
                        "sample_rate": { "type": "integer" },
                        "samples": { "type": "integer" },
                        "duration_ms": { "type": "integer" },
                        "word_timings": { "type": "array", "description": "With --option lipsync other than none." },
                        "lipsync": { "type": "object", "description": "With --option lipsync other than none: metadata and cues." }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            provider_flag(PROVIDERS, "Who speaks: piper (a local Piper voice). Literal, never {{ }}."),
            DslFlag { required: true, ..flag("--credential", "credential_id", "The provider's credential: for piper, kind tts, naming the store keys of the voice files.", "text") },
            DslFlag { required: true, ..flag("--text", "text", "What to say — a literal or {{ expr }}.", "text") },
            flag("--voice", "voice", "The voice: for piper, the speaker id of a multi-speaker model.", "text"),
            flag("--speed", "speed", "Speed factor: 1 is normal, greater is faster.", "number"),
            option_flag(),
            DslFlag { choices: words(RETURNS), ..flag("--return", "return", "file (default) writes a wav to the store and answers its FileRef; inline writes nothing and answers the bytes as base64. Literal.", "") },
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "Store folder for the audio file (default: audio).", "text"),
            flag("--filename", "filename", "Audio file name; .wav is added (default: a UUID).", "text"),
            flag("--path", "path", "Exact store key for the audio file; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        profiles: profiles(),
        fields: vec![
            NodeFieldDef {
                name: "provider".to_string(),
                label: "Provider".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![SelectOptionDef { value: "piper".to_string(), label: "Piper".to_string() }],
                help: Some("Who speaks. Piper runs a local voice named by a tts credential.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsAll),
                help: Some("The provider's credential; for piper, kind tts naming the voice files.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "return".to_string(),
                label: "Return".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "file".to_string(), label: "File".to_string() },
                    SelectOptionDef { value: "inline".to_string(), label: "Inline".to_string() },
                ],
                default_value: Some(json!("file")),
                help: Some("file writes a wav and answers its FileRef; inline answers the bytes as base64.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Store folder for the audio file (default: audio).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "filename".to_string(),
                label: "Filename".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Audio file name; .wav is added (default: a UUID).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path".to_string(),
                label: "Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("audio/narrator-demo.wav".to_string()),
                help: Some("Exact store key; overrides folder and filename.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "voice".to_string(),
                label: "Voice".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("For piper, the speaker id of a multi-speaker voice.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "speed".to_string(),
                label: "Speed".to_string(),
                field_type: NodeFieldType::Number,
                default_value: Some(json!(1.0)),
                help: Some("Speed factor. 1 = normal. Greater is faster.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "option".to_string(),
                label: "Provider options".to_string(),
                field_type: NodeFieldType::KeyValuePairs,
                help: Some("Settings of the chosen provider, key=value. Piper: volume (a number), lipsync (none, basic, timed_words, audio_guided, audio_segmented).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "text".to_string(),
                label: "Text".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(4),
                placeholder: Some("{{ input.script.text }}".to_string()),
                help: Some("What to say — a literal or {{ expr }}.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("provider".to_string()),
                    LayoutItem::Field("credential_id".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("return".to_string()),
                    LayoutItem::Field("folder".to_string()),
                    LayoutItem::Field("filename".to_string()),
                    LayoutItem::Field("path".to_string()),
                    LayoutItem::Field("store".to_string()),
                    LayoutItem::Field("on_conflict".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("voice".to_string()),
                    LayoutItem::Field("speed".to_string()),
                ],
            },
            LayoutItem::Field("option".to_string()),
            LayoutItem::Field("text".to_string()),
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Read a post aloud", r#"ai.audio.generate --provider piper --credential piper_en --text "{{ input.query.rows[0].body }}" --filename "{{ $trigger.params.slug }}""#)
                .output(serde_json::json!({ "audio": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "audio/hello.wav", "filename": "hello.wav", "mime": "audio/wav", "kind": "audio", "size": 88244, "sha256": "sha256:…", "lifecycle": "durable", "origin": "ai.audio.generate", "trust": "generated", "provider": "piper", "format": "wav", "mime_type": "audio/wav", "sample_rate": 22050, "samples": 44100, "duration_ms": 2000 } })),
            crate::pipeline::model::NodeExample::dsl("Bytes for a page, with mouth cues", r#"ai.audio.generate --provider piper --credential piper_en --text "Hello there." --return inline --option lipsync=timed_words"#)
                .note("`audio.base64` is the wav; `audio.word_timings` and `audio.lipsync.cues` drive an avatar's mouth. Nothing is written to the store."),
        ],
        ..Default::default()
    }
}

pub struct Node {
    config: Config,
    settings: Settings,
    credentials: Option<Arc<CredentialService>>,
    platform: Option<Arc<PlatformService>>,
}

impl Node {
    /// The node from its stored (or resolved) config: held to the chosen
    /// provider's profile, then read and checked once.
    pub fn build(
        config: &Value,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
    ) -> Result<Self, PipelineError> {
        check_config_profile(config)?;
        let config: Config =
            serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(CONFIG_CODE, err.to_string()))?;
        let settings = config.settings()?;
        Ok(Self { config, settings, credentials, platform })
    }

    /// The answer for what was synthesised: written to the store with
    /// `--return file`, or carried as base64 with `--return inline`.
    fn deliver(
        &self,
        platform: &Arc<PlatformService>,
        owner: &str,
        project: &str,
        synthesis: Synthesis,
    ) -> Result<Value, PipelineError> {
        let (word_timings, lipsync) = build_lipsync_payload(
            self.settings.lipsync,
            self.config.text.trim(),
            synthesis.duration_ms,
            synthesis.sample_rate,
            &synthesis.wav,
        )?;
        let mut audio = match self.settings.return_mode {
            ReturnMode::Inline => json!({
                "base64": BASE64_STANDARD.encode(&synthesis.wav),
                "size": synthesis.wav.len(),
            }),
            ReturnMode::File => {
                let folder = if self.config.folder.trim().is_empty() { "audio" } else { self.config.folder.trim() };
                let filename = self
                    .config
                    .filename
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let key = target_key(self.config.path.as_deref(), folder, &filename, "FW_NODE_AI_AUDIO_GENERATE_OUTPUT_PATH")?;
                let final_rel = normalize_audio_output_rel_path(&key)?;
                let store = open_store(platform, owner, project, self.config.store.as_deref())?;
                let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, FILE_CODE)?;
                if on_conflict.allows(&store.fs, &final_rel, FILE_CODE)? {
                    store.fs.put(&final_rel, &synthesis.wav).map_err(|err| {
                        PipelineError::new(FILE_CODE, format!("failed to write wav file: {err}"))
                    })?;
                    let leaf = final_rel.rsplit('/').next().unwrap_or(&final_rel).to_string();
                    store.file_ref(&final_rel, &leaf, "audio/wav", &synthesis.wav, NODE_KIND, "generated")
                } else {
                    // Skipped: the answer is the file already there, as it is.
                    store.stored_ref(&final_rel, NODE_KIND, "generated", FILE_CODE)?
                }
            }
        };
        audio["provider"] = json!(self.config.provider.trim());
        audio["format"] = json!("wav");
        audio["mime_type"] = json!("audio/wav");
        audio["sample_rate"] = json!(synthesis.sample_rate);
        audio["samples"] = json!(synthesis.samples);
        audio["duration_ms"] = json!(synthesis.duration_ms);
        if self.settings.lipsync != LipSyncMode::None {
            audio["word_timings"] = word_timings;
            audio["lipsync"] = lipsync;
        }
        Ok(json!({ "audio": audio }))
    }

    /// Piper: the credential's voice files pulled from the node's store into
    /// a scratch folder, then the Python bridge.
    fn synthesise_piper(
        &self,
        platform: &Arc<PlatformService>,
        owner: &str,
        project: &str,
        secret: &PiperCredentialSecret,
    ) -> Result<Synthesis, PipelineError> {
        let zebfs = open_store(platform, owner, project, self.config.store.as_deref())?.fs;
        let scratch = StoreScratch::new(FILE_CODE)?;
        let model_rel = normalize_zebfs_asset_rel_path(required_secret_str(&secret.model_file, "model_file")?)?;
        let config_rel = normalize_zebfs_asset_rel_path(required_secret_str(&secret.config_file, "config_file")?)?;
        let model_abs = scratch.pull(&zebfs, &model_rel).map_err(|_| {
            PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_MODEL", format!("model file '{model_rel}' does not exist"))
        })?;
        let config_abs = scratch.pull(&zebfs, &config_rel).map_err(|_| {
            PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_VOICE_CONFIG", format!("config file '{config_rel}' does not exist"))
        })?;
        let espeak_abs = match secret.espeak_data_dir.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            Some(value) => {
                let espeak_rel = normalize_zebfs_asset_rel_path(value)?;
                Some(scratch.pull(&zebfs, &espeak_rel).map_err(|_| {
                    PipelineError::new(
                        "FW_NODE_AI_AUDIO_GENERATE_ESPEAK",
                        format!("espeak data dir '{espeak_rel}' does not exist"),
                    )
                })?)
            }
            None => None,
        };
        let bridge = run_piper_bridge(&PiperBridgeRequest {
            model_path: model_abs,
            config_path: config_abs,
            espeak_data_dir: espeak_abs,
            text: self.config.text.trim().to_string(),
            speaker: self.settings.voice,
            length_scale: speed_to_length_scale(self.settings.speed)?,
            volume: self.settings.volume,
        })?;
        let wav = BASE64_STANDARD.decode(bridge.audio_blob_base64.as_bytes()).map_err(|err| {
            PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_BLOB", format!("failed to decode audio blob: {err}"))
        })?;
        Ok(Synthesis { wav, sample_rate: bridge.sample_rate, samples: bridge.samples, duration_ms: bridge.duration_ms })
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
                "FW_NODE_AI_AUDIO_GENERATE_PLATFORM",
                "platform service is not configured on this framework engine",
            )
        })?;
        let credentials = self.credentials.as_ref().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_AI_AUDIO_GENERATE_CREDENTIALS",
                "credential service is not configured on this framework engine",
            )
        })?;
        let provider = self.config.provider.trim();

        // Arrives final — `{{ }}` resolved engine-side before this ran.
        if self.config.text.trim().is_empty() {
            return Err(PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_TEXT", "--text is empty; it needs what to say"));
        }

        let credential_id = self.config.credential_id.trim();
        if credential_id.is_empty() {
            return Err(PipelineError::new(CREDENTIAL_CODE, "--credential is empty; it needs the provider's credential"));
        }
        let credential = credentials
            .get_project_credential(owner, project, credential_id)
            .map_err(|err| PipelineError::new(CREDENTIAL_CODE, err.to_string()))?
            .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, format!("credential '{credential_id}' not found")))?;
        check_credential(checked_definition(), provider, credential_id, &credential.kind)
            .map_err(|message| PipelineError::new(CREDENTIAL_CODE, message))?;

        let secret: PiperCredentialSecret = serde_json::from_value(credential.secret.clone()).map_err(|err| {
            PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_CREDENTIAL_SECRET", format!("invalid tts credential secret: {err}"))
        })?;
        if let Some(secret_provider) = secret.provider.as_deref() {
            let secret_provider = secret_provider.trim().to_lowercase();
            if !secret_provider.is_empty() && secret_provider != provider {
                return Err(PipelineError::new(
                    "FW_NODE_AI_AUDIO_GENERATE_PROVIDER_MISMATCH",
                    format!("--provider '{provider}' does not match credential provider '{secret_provider}'"),
                ));
            }
        }

        let synthesis = self.synthesise_piper(platform, owner, project, &secret)?;
        let answer = self.deliver(platform, owner, project, synthesis)?;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, answer),
            trace: vec![format!(
                "node_kind={NODE_KIND} provider={provider} credential={credential_id} lipsync={}",
                self.settings.lipsync.as_str()
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
            PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_PIPER", format!("failed to start python3: {err}"))
        })?;

    {
        let Some(stdin) = child.stdin.as_mut() else {
            return Err(PipelineError::new(
                "FW_NODE_AI_AUDIO_GENERATE_PIPER",
                "failed to open python stdin",
            ));
        };
        let payload = serde_json::to_vec(req).map_err(|err| {
            PipelineError::new(
                "FW_NODE_AI_AUDIO_GENERATE_PIPER",
                format!("failed to serialize request: {err}"),
            )
        })?;
        stdin.write_all(&payload).map_err(|err| {
            PipelineError::new(
                "FW_NODE_AI_AUDIO_GENERATE_PIPER",
                format!("failed to write python stdin: {err}"),
            )
        })?;
    }

    let output = child.wait_with_output().map_err(|err| {
        PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_PIPER", format!("failed waiting for python: {err}"))
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
        return Err(PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_PIPER", detail));
    }

    serde_json::from_slice::<PiperBridgeResult>(&output.stdout).map_err(|err| {
        PipelineError::new(
            "FW_NODE_AI_AUDIO_GENERATE_PIPER",
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
                "FW_NODE_AI_AUDIO_GENERATE_CREDENTIAL_SECRET",
                format!("tts credential secret must include non-empty '{field}'"),
            )
        })
}

fn speed_to_length_scale(speed: f32) -> Result<Option<f32>, PipelineError> {
    if !speed.is_finite() || speed <= 0.0 {
        return Err(PipelineError::new(
            CONFIG_CODE,
            "--speed must be a number greater than 0",
        ));
    }
    if (speed - 1.0).abs() < f32::EPSILON {
        Ok(None)
    } else {
        Ok(Some(1.0 / speed))
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


fn normalize_zebfs_asset_rel_path(raw: &str) -> Result<String, PipelineError> {
    let normalized = raw.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err(PipelineError::new("FW_NODE_AI_AUDIO_GENERATE_PATH", "path must not be empty"));
    }
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "FW_NODE_AI_AUDIO_GENERATE_PATH",
                "path must stay inside project Zebflow FS",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_AI_AUDIO_GENERATE_PATH",
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
            "FW_NODE_AI_AUDIO_GENERATE_OUTPUT_PATH",
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
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Instant;

    use serde_json::json;

    use super::{
        LipSyncMode, Node, Synthesis, WordTiming, basic_word_timings,
        build_audio_segmented_cues, expand_audio_guided_cues_for_word,
        normalize_audio_output_rel_path, reconcile_word_boundaries,
        speed_to_length_scale, split_word_into_viseme_segments, tokenize_words,
        weighted_word_timings,
    };
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::{PlatformConfig, UpsertProjectCredentialRequest};
    use crate::platform::services::PlatformService;

    /// The Piper voice (`ZEBFLOW_TEST_PIPER_MODEL`, its `.onnx.json` beside it)
    /// and the espeak-ng data folder (`ZEBFLOW_TEST_ESPEAK_DATA`) the manual
    /// smoke tests read. `None` when either is unset.
    fn local_piper_assets() -> Option<(PathBuf, PathBuf, PathBuf)> {
        let model = PathBuf::from(std::env::var_os("ZEBFLOW_TEST_PIPER_MODEL")?);
        let espeak = PathBuf::from(std::env::var_os("ZEBFLOW_TEST_ESPEAK_DATA")?);
        let mut config = model.clone().into_os_string();
        config.push(".json");
        Some((model, PathBuf::from(config), espeak))
    }

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

    fn build(config: serde_json::Value) -> Result<Node, crate::pipeline::PipelineError> {
        Node::build(&config, None, None)
    }

    /// The profile closes `--provider` and `--option`; `--return` takes its
    /// two words; a mistyped option value is refused naming its type.
    #[test]
    fn the_profile_and_the_flags_are_checked_when_the_node_is_built() {
        assert!(build(json!({ "provider": "piper", "credential_id": "c", "text": "hi" })).is_ok());
        for (config, says) in [
            (json!({ "provider": "polly", "text": "hi" }), "'polly' is not one of piper"),
            (json!({ "provider": "piper", "text": "hi", "option": { "pitch": "2" } }), "'pitch' is not a setting of --provider piper; it takes volume, lipsync"),
            (json!({ "provider": "piper", "text": "hi", "option": { "volume": "loud" } }), "--option volume='loud' is not a number"),
            (json!({ "provider": "piper", "text": "hi", "option": { "lipsync": "timed" } }), "lipsync=none|basic|timed_words|audio_guided|audio_segmented"),
            (json!({ "provider": "piper", "text": "hi", "return": "both" }), "--return 'both' must be one of inline, file"),
            (json!({ "provider": "piper", "text": "hi", "voice": "alto" }), "--voice 'alto' is not a number"),
            (json!({ "provider": "piper", "text": "hi", "speed": "0" }), "--speed must be a number greater than 0"),
        ] {
            let err = build(config).err().expect("refused");
            assert_eq!(err.code, "FW_NODE_AI_AUDIO_GENERATE_CONFIG");
            assert!(err.message.contains(says), "{}", err.message);
        }
        let node = build(json!({ "provider": "piper", "text": "hi", "voice": 3, "option": { "volume": "0.5", "lipsync": "basic" } })).expect("node");
        assert_eq!(node.settings.voice, Some(3));
        assert!((node.settings.volume - 0.5).abs() < f32::EPSILON);
        assert_eq!(node.settings.lipsync, LipSyncMode::Basic);
    }

    /// The piper signature, generated from the definition and its profile.
    #[test]
    fn the_piper_signature_is_generated() {
        assert_eq!(
            crate::pipeline::nodes::provider_signature(&super::definition(), "piper").as_deref(),
            Some("ai.audio.generate --provider piper --credential TEXT --text TEXT [--voice TEXT] [--speed N] [--option volume=N] [--option lipsync=none|basic|timed_words|audio_guided|audio_segmented] [--return inline|file] [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → audio")
        );
    }

    /// A short silent wav, standing in for what a provider synthesised.
    fn synthesis() -> Synthesis {
        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&[0u8; 40]);
        Synthesis { wav, sample_rate: 22050, samples: 22, duration_ms: 1 }
    }

    /// `--return file` writes the wav and answers its FileRef under `audio`;
    /// `--return inline` writes nothing and answers the bytes. The payload is
    /// kept either way.
    #[test]
    fn the_answer_is_audio_as_a_file_or_inline() {
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        let file = build(json!({ "provider": "piper", "credential_id": "c", "text": "hi there", "filename": "greeting", "option": { "lipsync": "timed_words" } })).expect("node");
        let answer = file.deliver(&platform, "superadmin", "default", synthesis()).expect("written");
        let payload = crate::pipeline::nodes::shared::util::with_answer(&json!({ "kept": 1 }), answer);
        assert_eq!(payload["kept"], 1);
        let audio = &payload["audio"];
        assert_eq!(audio["__zf_type"], "file_ref");
        assert_eq!(audio["ref"], "audio/greeting.wav");
        assert_eq!(audio["provider"], "piper");
        assert_eq!(audio["mime_type"], "audio/wav");
        assert!(audio["word_timings"].as_array().is_some_and(|w| w.len() == 2), "{audio}");
        assert!(audio.get("base64").is_none());
        let store = crate::pipeline::nodes::shared::project_store::open_store(&platform, "superadmin", "default", None).expect("store");
        assert!(store.fs.head("audio/greeting.wav").is_ok(), "the wav is in the store");

        let inline = build(json!({ "provider": "piper", "credential_id": "c", "text": "hi", "filename": "unused", "return": "inline" })).expect("node");
        let answer = inline.deliver(&platform, "superadmin", "default", synthesis()).expect("answered");
        let payload = crate::pipeline::nodes::shared::util::with_answer(&json!({ "kept": 1 }), answer);
        assert_eq!(payload["kept"], 1);
        let audio = &payload["audio"];
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, audio["base64"].as_str().expect("base64")).expect("decodes");
        assert!(bytes.starts_with(b"RIFF"));
        assert_eq!(audio["size"], 44);
        assert!(audio.get("ref").is_none() && audio.get("word_timings").is_none(), "{audio}");
        assert!(store.fs.head("audio/unused.wav").is_err(), "inline writes nothing");
    }

    /// A credential of a kind the provider's profile does not name is refused
    /// before anything runs.
    #[tokio::test]
    async fn a_credential_of_another_kind_is_refused() {
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "llm".to_string(),
                    title: "LLM".to_string(),
                    kind: "openai".to_string(),
                    secret: json!({ "api_key": uuid::Uuid::new_v4().to_string() }),
                    notes: String::new(),
                },
            )
            .expect("credential");
        let node = Node::build(
            &json!({ "provider": "piper", "credential_id": "llm", "text": "hi" }),
            Some(platform.credentials.clone()),
            Some((*platform).clone()),
        )
        .expect("node");
        let err = node
            .execute_async(NodeExecutionInput {
                node_id: "a".to_string(),
                input_pin: "in".to_string(),
                payload: json!({}),
                metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
                bus: None,
            })
            .await
            .err()
            .expect("refused");
        assert_eq!(err.code, "FW_NODE_AI_AUDIO_GENERATE_CREDENTIAL");
        assert!(err.message.contains("is kind 'openai'; --provider piper takes a credential of kind tts"), "{}", err.message);
    }

    /// One word per mode: the short aliases are gone.
    #[test]
    fn lipsync_mode_takes_one_word_per_mode() {
        let parse = |raw: &str| {
            serde_json::from_value::<LipSyncMode>(serde_json::json!(raw)).expect("mode")
        };
        assert_eq!(parse("none"), LipSyncMode::None);
        assert_eq!(parse("basic"), LipSyncMode::Basic);
        assert_eq!(parse("timed_words"), LipSyncMode::TimedWords);
        assert_eq!(parse("audio_guided"), LipSyncMode::AudioGuided);
        assert_eq!(parse("audio_segmented"), LipSyncMode::AudioSegmented);
        assert!(serde_json::from_value::<LipSyncMode>(serde_json::json!("timed")).is_err());
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
    #[ignore = "manual local smoke using Piper voice assets"]
    async fn piper_node_smoke_with_local_voice() {
        let Some((model_src, config_src, espeak_src)) = local_piper_assets() else {
            eprintln!("ZEBFLOW_TEST_PIPER_MODEL / ZEBFLOW_TEST_ESPEAK_DATA are not set; skipping");
            return;
        };
        if !(model_src.is_file() && config_src.is_file() && espeak_src.is_dir()) {
            eprintln!("local Piper smoke assets are not present; skipping");
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

        let voice_dir = layout.files_dir.join("voices/narrator");
        fs::create_dir_all(&voice_dir).expect("voice dir");
        fs::copy(&model_src, voice_dir.join("narrator.onnx")).expect("copy model");
        fs::copy(&config_src, voice_dir.join("narrator.onnx.json")).expect("copy config");
        copy_dir_all(&espeak_src, &layout.files_dir.join("runtime/espeak-ng-data"))
            .expect("copy espeak");

        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "narrator-tts".to_string(),
                    title: "Narrator TTS".to_string(),
                    kind: "tts".to_string(),
                    secret: json!({
                        "provider": "piper",
                        "model_file": "voices/narrator/narrator.onnx",
                        "config_file": "voices/narrator/narrator.onnx.json",
                        "espeak_data_dir": "runtime/espeak-ng-data",
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let node = Node::build(
            &json!({
                "provider": "piper",
                "credential_id": "narrator-tts",
                "text": "Halo, ini Narrator dari node ai.audio.generate Zebflow.",
                "path": "audio/narrator-node-smoke.wav",
                "on_conflict": "overwrite",
                "option": { "lipsync": "basic" }
            }),
            Some(platform.credentials.clone()),
            Some(platform.clone()),
        )
        .expect("node");

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
                bus: None,
            })
            .await
            .expect("execute");

        let file_rel = out.payload["audio"]["ref"]
            .as_str()
            .expect("audio file should be present");
        assert_eq!(file_rel, "audio/narrator-node-smoke.wav");
        assert!(
            layout.files_dir.join(file_rel).is_file(),
            "expected synthesized wav file to exist"
        );
        assert_eq!(
            out.payload["audio"]["lipsync"]["metadata"]["phoneme_method"],
            json!("basic")
        );
        assert!(
            out.payload["audio"]["word_timings"]
                .as_array()
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        );
    }

    #[tokio::test]
    #[ignore = "manual local benchmark using Piper voice assets"]
    async fn piper_lipsync_benchmark_with_local_voice() {
        let Some((model_src, config_src, espeak_src)) = local_piper_assets() else {
            eprintln!("ZEBFLOW_TEST_PIPER_MODEL / ZEBFLOW_TEST_ESPEAK_DATA are not set; skipping");
            return;
        };
        if !(model_src.is_file() && config_src.is_file() && espeak_src.is_dir()) {
            eprintln!("local Piper benchmark assets are not present; skipping");
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

        let voice_dir = layout.files_dir.join("voices/narrator");
        fs::create_dir_all(&voice_dir).expect("voice dir");
        fs::copy(&model_src, voice_dir.join("narrator.onnx")).expect("copy model");
        fs::copy(&config_src, voice_dir.join("narrator.onnx.json")).expect("copy config");
        copy_dir_all(&espeak_src, &layout.files_dir.join("runtime/espeak-ng-data"))
            .expect("copy espeak");

        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &UpsertProjectCredentialRequest {
                    credential_id: "narrator-tts".to_string(),
                    title: "Narrator TTS".to_string(),
                    kind: "tts".to_string(),
                    secret: json!({
                        "provider": "piper",
                        "model_file": "voices/narrator/narrator.onnx",
                        "config_file": "voices/narrator/narrator.onnx.json",
                        "espeak_data_dir": "runtime/espeak-ng-data",
                    }),
                    notes: String::new(),
                },
            )
            .expect("credential");

        let sentences = vec![
            "Halo, saya sedang mencoba suara Narrator untuk demo Zebflow hari ini.",
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

        let warmup = Node::build(
            &json!({
                "provider": "piper",
                "credential_id": "narrator-tts",
                "text": "Warmup untuk benchmark lipsync Zebflow.",
                "return": "inline"
            }),
            Some(platform.credentials.clone()),
            Some(platform.clone()),
        )
        .expect("node");
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
                bus: None,
            })
            .await
            .expect("warmup");

        eprintln!("| Sentence | none | basic | timed_words | audio_guided |");
        eprintln!("|---|---:|---:|---:|---:|");
        for sentence in sentences {
            let mut row = vec![sentence.to_string()];
            for mode in modes {
                let node = Node::build(
                    &json!({
                        "provider": "piper",
                        "credential_id": "narrator-tts",
                        "text": sentence,
                        "return": "inline",
                        "option": { "lipsync": mode.as_str() }
                    }),
                    Some(platform.credentials.clone()),
                    Some(platform.clone()),
                )
                .expect("node");

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
                        bus: None,
                    })
                    .await
                    .expect("execute");
                let elapsed_ms = started.elapsed().as_millis();
                if mode != LipSyncMode::None {
                    assert!(out.payload["audio"]["lipsync"].is_object());
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
