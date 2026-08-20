//! Permanent project configuration contract.
//!
//! The structures in this module own the repository format. Runtime services
//! continue to use [`ZebflowJson`], with explicit conversions at the boundary,
//! so ordinary Rust refactoring cannot silently change `repo/zebflow.yaml`.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contracts::{ContractError, ContractKind, ContractMetadata, PlatformContract};
use crate::infra::execution::backend::ExecutionProfileKind;
use crate::infra::execution::placement::{
    ProjectRuntimeMode, ProjectRuntimeProfile, ResourceProfile, RuntimeResourceSpec,
};
use crate::infra::execution::sync::ProjectBootstrapPlan;
use crate::platform::model::{
    ZebflowJson, ZebflowJsonAssistant, ZebflowJsonConfigs, ZebflowJsonData,
    ZebflowJsonDistribution, ZebflowJsonDistributionHub, ZebflowJsonFiles, ZebflowJsonGit,
    ZebflowJsonGitRemote, ZebflowJsonInitialDataDir, ZebflowJsonLayout, ZebflowJsonLocks,
    ZebflowJsonLogging, ZebflowJsonMetadata, ZebflowJsonPipelines, ZebflowJsonRwe,
    ZebflowJsonRweLibraryEntry, ZebflowJsonUploads, slug_segment,
};

/// Canonical repository filename for project configuration.
pub const PROJECT_CONFIGURATION_FILE: &str = "zebflow.yaml";
/// Previous repository filename accepted only by the explicit migration path.
pub const LEGACY_PROJECT_CONFIGURATION_FILE: &str = "zebflow.json";
/// Recovery copy retained by the explicit JSON-to-YAML migration.
pub const PROJECT_CONFIGURATION_BACKUP_FILE: &str = "zebflow.pre-yaml.json";

/// Canonical `zebflow.yaml` contract.
pub struct ProjectConfigurationContract;

impl PlatformContract for ProjectConfigurationContract {
    type Spec = ProjectConfigurationSpec;
    const KIND: ContractKind = ContractKind::ProjectConfiguration;

    fn validate(metadata: &ContractMetadata, spec: &Self::Spec) -> Result<(), ContractError> {
        if metadata.name != slug_segment(&metadata.name) {
            return Err(ContractError::invalid(
                "metadata.name must be a canonical project slug",
            ));
        }
        if metadata.version.is_some()
            || metadata.digest.is_some()
            || !metadata.annotations.is_empty()
        {
            return Err(ContractError::invalid(
                "ProjectConfiguration metadata contains only name",
            ));
        }
        spec.validate()
    }
}

/// Portable, non-secret project configuration stored under the contract envelope.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfigurationSpec {
    #[serde(default)]
    pub profile: ProjectProfileSpec,
    /// Repository layout. Unlike the other sections this one is omitted when it
    /// is empty, so a project that never declared a layout is not rewritten to
    /// carry an empty section it did not author.
    #[serde(default, skip_serializing_if = "is_default")]
    pub layout: ProjectLayoutSpec,
    #[serde(default)]
    pub rwe: ProjectRweSpec,
    #[serde(default)]
    pub pipelines: ProjectPipelinesSpec,
    #[serde(default)]
    pub runtime: ProjectRuntimeSpec,
    #[serde(default)]
    pub bootstrap: ProjectBootstrapSpec,
    #[serde(default)]
    pub git: ProjectGitSpec,
    #[serde(default)]
    pub assistant: ProjectAssistantSpec,
    #[serde(default)]
    pub locks: ProjectLocksSpec,
    /// Reserved for future project-wide data behavior. It never contains records.
    #[serde(default)]
    pub data: ProjectDataSpec,
    #[serde(default)]
    pub files: ProjectFilesSpec,
    #[serde(default)]
    pub distribution: ProjectDistributionSpec,
}

impl ProjectConfigurationSpec {
    fn validate(&self) -> Result<(), ContractError> {
        self.layout.validate()?;
        validate_text("spec.profile.title", &self.profile.title, 256, false)?;
        validate_text(
            "spec.profile.description",
            &self.profile.description,
            16_384,
            true,
        )?;
        for url in &self.rwe.allow_list {
            validate_http_pattern("spec.rwe.allow_list", url)?;
        }
        if self
            .rwe
            .allow_list
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != self.rwe.allow_list.len()
        {
            return Err(ContractError::invalid(
                "spec.rwe.allow_list must contain unique entries",
            ));
        }
        if let Some(base) = &self.rwe.deployment_asset_base {
            validate_http_or_root_path("spec.rwe.deployment_asset_base", base)?;
        }
        for (name, library) in &self.rwe.libraries {
            if !name.starts_with("zeb/") || !is_safe_logical_name(name, true) {
                return Err(ContractError::invalid(format!(
                    "spec.rwe.libraries key '{name}' must use the zeb/<name> form"
                )));
            }
            validate_text("spec.rwe.libraries.*.version", &library.version, 128, false)?;
            if library.version.trim().is_empty() {
                return Err(ContractError::invalid(
                    "spec.rwe.libraries.*.version must not be empty",
                ));
            }
            if !matches!(library.source.as_str(), "offline" | "online") {
                return Err(ContractError::invalid(
                    "spec.rwe.libraries.*.source must be offline or online",
                ));
            }
        }
        if let Some(value) = self.pipelines.logging.max_invocations
            && !(1..=1000).contains(&value)
        {
            return Err(ContractError::invalid(
                "spec.pipelines.logging.max_invocations must be between 1 and 1000",
            ));
        }
        if let Some(value) = self.pipelines.node_timeout_secs
            && !(5..=3600).contains(&value)
        {
            return Err(ContractError::invalid(
                "spec.pipelines.node_timeout_secs must be between 5 and 3600",
            ));
        }

        validate_optional_mb(
            "spec.files.uploads.max_asset_size_mb",
            self.files.uploads.max_asset_size_mb,
            5,
        )?;
        validate_optional_mb(
            "spec.files.uploads.webhook_body_max_mb",
            self.files.uploads.webhook_body_max_mb,
            100,
        )?;
        validate_optional_mb(
            "spec.files.uploads.max_file_size_mb",
            self.files.uploads.max_file_size_mb,
            5,
        )?;

        if self.runtime.min_replicas == 0 {
            return Err(ContractError::invalid(
                "spec.runtime.min_replicas must be greater than zero",
            ));
        }
        if let Some(max) = self.runtime.max_replicas
            && max < self.runtime.min_replicas
        {
            return Err(ContractError::invalid(
                "spec.runtime.max_replicas must be greater than or equal to min_replicas",
            ));
        }
        let mut runtime_tags = std::collections::BTreeSet::new();
        for tag in &self.runtime.required_tags {
            if !is_safe_logical_name(tag, false) || !runtime_tags.insert(tag) {
                return Err(ContractError::invalid(
                    "spec.runtime.required_tags must contain unique logical names",
                ));
            }
        }
        match (
            self.runtime.resource_profile,
            self.runtime.custom_resources.as_ref(),
        ) {
            (ProjectResourceProfileSpec::Custom, None) => {
                return Err(ContractError::invalid(
                    "spec.runtime.custom_resources is required for the custom resource profile",
                ));
            }
            (ProjectResourceProfileSpec::Custom, Some(resources)) => {
                resources.validate()?;
            }
            (_, Some(_)) => {
                return Err(ContractError::invalid(
                    "spec.runtime.custom_resources is only allowed for the custom resource profile",
                ));
            }
            (_, None) => {}
        }

        validate_unique_paths("spec.bootstrap.activate", &self.bootstrap.activate, true)?;
        validate_git(&self.git.remote)?;
        for (path, value) in [
            (
                "spec.assistant.high_model_credential",
                self.assistant.high_model_credential.as_deref(),
            ),
            (
                "spec.assistant.general_model_credential",
                self.assistant.general_model_credential.as_deref(),
            ),
        ] {
            if let Some(value) = value {
                validate_identifier(path, value)?;
            }
        }
        validate_optional_range(
            "spec.assistant.max_steps",
            self.assistant.max_steps,
            1,
            1000,
        )?;
        validate_optional_range(
            "spec.assistant.max_replans",
            self.assistant.max_replans,
            0,
            64,
        )?;
        validate_optional_range(
            "spec.assistant.chat_history_pairs",
            self.assistant.chat_history_pairs,
            0,
            50,
        )?;
        validate_unique_paths("spec.locks.templates", &self.locks.templates, false)?;
        if !self.distribution.hub.entry_url.is_empty() {
            validate_http_or_root_path(
                "spec.distribution.hub.entry_url",
                &self.distribution.hub.entry_url,
            )?;
        }
        Ok(())
    }
}

fn validate_text(
    path: &str,
    value: &str,
    maximum: usize,
    allow_newlines: bool,
) -> Result<(), ContractError> {
    if value.len() > maximum
        || value.chars().any(|character| {
            character == '\0'
                || (character.is_control()
                    && (!allow_newlines || !matches!(character, '\n' | '\r' | '\t')))
        })
    {
        return Err(ContractError::invalid(format!(
            "{path} must be at most {maximum} bytes and contain no unsupported control characters"
        )));
    }
    Ok(())
}

fn validate_identifier(path: &str, value: &str) -> Result<(), ContractError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ContractError::invalid(format!(
            "{path} must contain only letters, numbers, dot, underscore, and hyphen"
        )));
    }
    Ok(())
}

fn is_safe_logical_name(value: &str, allow_slash: bool) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.')
                || (allow_slash && byte == b'/')
        })
        && !value
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
}

fn validate_http_pattern(path: &str, value: &str) -> Result<(), ContractError> {
    if value.len() > 2048
        || value.chars().any(char::is_control)
        || !(value.starts_with("https://") || value.starts_with("http://"))
        || value.split_once("://").is_some_and(|(_, rest)| {
            rest.split('/')
                .next()
                .is_some_and(|authority| authority.contains('@'))
        })
    {
        return Err(ContractError::invalid(format!(
            "{path} entries must be HTTP(S) patterns without embedded credentials"
        )));
    }
    Ok(())
}

fn validate_http_or_root_path(path: &str, value: &str) -> Result<(), ContractError> {
    if value.starts_with('/') {
        if value.starts_with("//") || value.contains("..") || value.chars().any(char::is_control) {
            return Err(ContractError::invalid(format!(
                "{path} must be a normalized root-relative path"
            )));
        }
        return Ok(());
    }
    let url = reqwest::Url::parse(value)
        .map_err(|_| ContractError::invalid(format!("{path} must be HTTP(S) or root-relative")))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ContractError::invalid(format!(
            "{path} must be HTTP(S) without embedded credentials"
        )));
    }
    Ok(())
}

fn validate_git(remote: &ProjectGitRemoteSpec) -> Result<(), ContractError> {
    let empty =
        remote.credential_id.is_empty() && remote.repo_url.is_empty() && remote.branch.is_empty();
    if empty {
        return Ok(());
    }
    if remote.repo_url.is_empty() || remote.branch.is_empty() {
        return Err(ContractError::invalid(
            "spec.git.remote.repo_url and branch are required together",
        ));
    }
    if !remote.credential_id.is_empty() {
        validate_identifier("spec.git.remote.credential_id", &remote.credential_id)?;
    }
    validate_git_url(&remote.repo_url)?;
    validate_git_branch(&remote.branch)
}

fn validate_git_url(value: &str) -> Result<(), ContractError> {
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return Err(ContractError::invalid(
            "spec.git.remote.repo_url is invalid",
        ));
    }
    if value.starts_with("git@") {
        if value.contains(':') && !value.contains(char::is_whitespace) {
            return Ok(());
        }
    }
    let url = reqwest::Url::parse(value)
        .map_err(|_| ContractError::invalid("spec.git.remote.repo_url is invalid"))?;
    let invalid_credentials =
        url.password().is_some() || (url.scheme() == "https" && !url.username().is_empty());
    if !matches!(url.scheme(), "https" | "ssh") || invalid_credentials {
        return Err(ContractError::invalid(
            "spec.git.remote.repo_url must use HTTPS or SSH without embedded credentials",
        ));
    }
    Ok(())
}

fn validate_git_branch(value: &str) -> Result<(), ContractError> {
    let invalid = value.is_empty()
        || value.len() > 255
        || value.starts_with('.')
        || value.ends_with('.')
        || value.ends_with('/')
        || value.ends_with(".lock")
        || value.contains("..")
        || value.contains("@{")
        || value
            .chars()
            .any(|character| character.is_control() || " ~^:?*[\\".contains(character));
    if invalid {
        return Err(ContractError::invalid(
            "spec.git.remote.branch is not a valid Git branch name",
        ));
    }
    Ok(())
}

fn validate_optional_range(
    path: &str,
    value: Option<u32>,
    minimum: u32,
    maximum: u32,
) -> Result<(), ContractError> {
    if value.is_some_and(|value| value < minimum || value > maximum) {
        return Err(ContractError::invalid(format!(
            "{path} must be between {minimum} and {maximum}"
        )));
    }
    Ok(())
}

fn validate_unique_paths(
    path: &str,
    values: &[String],
    allow_glob: bool,
) -> Result<(), ContractError> {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        let invalid_character = value.chars().any(|character| {
            character.is_control()
                || character == '\\'
                || (!allow_glob && matches!(character, '*' | '?' | '[' | ']'))
        });
        if value.is_empty()
            || value.len() > 1024
            || value.starts_with('/')
            || value
                .split('/')
                .any(|segment| segment == "." || segment == "..")
            || invalid_character
            || !seen.insert(value)
        {
            return Err(ContractError::invalid(format!(
                "{path} must contain unique, project-relative paths"
            )));
        }
    }
    Ok(())
}

/// Decodes the known pre-contract `zebflow.json` shape for explicit migration.
/// Normal project configuration reads never call this function.
pub fn decode_legacy_project_configuration(
    bytes: &[u8],
) -> Result<ProjectConfigurationSpec, ContractError> {
    let legacy: LegacyProjectConfiguration =
        serde_json::from_slice(bytes).map_err(ContractError::Json)?;
    if legacy.version != "1.0" {
        return Err(ContractError::invalid(format!(
            "unsupported legacy project configuration version '{}'",
            legacy.version
        )));
    }
    if !legacy.configs.pipelines.nodes.is_empty() {
        return Err(ContractError::invalid(
            "legacy configs.pipelines.nodes must be empty before migration",
        ));
    }
    let LegacyProjectConfigs {
        rwe,
        pipelines,
        runtime,
        bootstrap,
        git,
        assistant,
        locks,
        data,
        files,
    } = legacy.configs;
    let hub = match (legacy.distribution.marketplace, legacy.distribution.hub) {
        (Some(_), Some(_)) => {
            return Err(ContractError::invalid(
                "legacy distribution cannot contain both marketplace and hub",
            ));
        }
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => ZebflowJsonDistributionHub::default(),
    };
    let spec: ProjectConfigurationSpec = ZebflowJson {
        metadata: legacy.metadata,
        configs: ZebflowJsonConfigs {
            // The legacy shape has no layout section, so a migrated project
            // declares nothing and keeps the directories it already used.
            layout: ZebflowJsonLayout::default(),
            rwe,
            pipelines: ZebflowJsonPipelines {
                logging: pipelines.logging,
                node_timeout_secs: pipelines.node_timeout_secs,
            },
            runtime,
            bootstrap,
            git,
            assistant,
            locks,
            data,
            files,
        },
        distribution: ZebflowJsonDistribution { hub },
    }
    .into();
    spec.validate()?;
    Ok(spec)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProjectConfiguration {
    version: String,
    #[serde(default)]
    metadata: ZebflowJsonMetadata,
    #[serde(default)]
    configs: LegacyProjectConfigs,
    #[serde(default)]
    distribution: LegacyProjectDistribution,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LegacyProjectConfigs {
    #[serde(default)]
    rwe: ZebflowJsonRwe,
    #[serde(default)]
    pipelines: LegacyProjectPipelines,
    #[serde(default)]
    runtime: ProjectRuntimeProfile,
    #[serde(default)]
    bootstrap: ProjectBootstrapPlan,
    #[serde(default)]
    git: ZebflowJsonGit,
    #[serde(default)]
    assistant: ZebflowJsonAssistant,
    #[serde(default)]
    locks: ZebflowJsonLocks,
    #[serde(default)]
    data: ZebflowJsonData,
    #[serde(default)]
    files: ZebflowJsonFiles,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LegacyProjectPipelines {
    #[serde(default)]
    logging: ZebflowJsonLogging,
    #[serde(default)]
    node_timeout_secs: Option<u64>,
    #[serde(default)]
    nodes: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct LegacyProjectDistribution {
    #[serde(default)]
    marketplace: Option<ZebflowJsonDistributionHub>,
    #[serde(default)]
    hub: Option<ZebflowJsonDistributionHub>,
}

fn validate_optional_mb(path: &str, value: Option<u32>, minimum: u32) -> Result<(), ContractError> {
    if let Some(value) = value
        && !(minimum..=1024).contains(&value)
    {
        return Err(ContractError::invalid(format!(
            "{path} must be between {minimum} and 1024"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectProfileSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// Declared repository layout.
///
/// Every entry is optional. An absent entry resolves to the platform default in
/// [`ZebflowJsonLayout::resolve`], which is what keeps a project written before
/// this section existed on exactly the directories it already used.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectLayoutSpec {
    /// Source root: pipelines, pages, styles, and shared components. It is also
    /// the RWE template root, which is one directory today and not two.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    /// Directory holding the portable SQLite schema export.
    ///
    /// This is a separate entry from `schema` because the two exports are
    /// different documents produced by different engines, and a project may
    /// carry one without the other.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sqlite_schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_interfaces: Option<String>,
    /// Prefixes an install replays as initial data, each bound to one engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_data: Option<Vec<ProjectInitialDataDirSpec>>,
}

/// One initial-data prefix and the database engine that would replay it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectInitialDataDirSpec {
    pub path: String,
    pub engine: String,
}

/// Engine tokens the installer dispatches on. A prefix bound to anything else
/// would name a replay path that no code performs.
const INITIAL_DATA_ENGINES: &[&str] = &["sekejap", "sqlite"];

impl ProjectLayoutSpec {
    fn validate(&self) -> Result<(), ContractError> {
        for (path, value) in [
            ("spec.layout.source", self.source.as_deref()),
            ("spec.layout.assets", self.assets.as_deref()),
            ("spec.layout.docs", self.docs.as_deref()),
            ("spec.layout.schema", self.schema.as_deref()),
            ("spec.layout.sqlite_schema", self.sqlite_schema.as_deref()),
            (
                "spec.layout.node_interfaces",
                self.node_interfaces.as_deref(),
            ),
        ] {
            if let Some(value) = value {
                validate_layout_dir(path, value)?;
            }
        }
        let Some(entries) = &self.initial_data else {
            return Ok(());
        };
        let mut seen = std::collections::BTreeSet::new();
        for entry in entries {
            validate_layout_dir("spec.layout.initial_data.*.path", &entry.path)?;
            if !INITIAL_DATA_ENGINES.contains(&entry.engine.as_str()) {
                return Err(ContractError::invalid(format!(
                    "spec.layout.initial_data.*.engine must be one of {}",
                    INITIAL_DATA_ENGINES.join(", ")
                )));
            }
            if !seen.insert(&entry.path) {
                return Err(ContractError::invalid(
                    "spec.layout.initial_data paths must be unique",
                ));
            }
        }
        Ok(())
    }
}

/// Accepts one normalized, project-relative directory.
///
/// Layout entries name directories that other paths are built on top of, so a
/// trailing slash or a traversal segment here would be inherited by every path
/// derived from it rather than failing at the one place it was written.
///
/// `HubPackage` records the layout a publisher's paths were produced by and
/// validates it through this same rule, because a package layout an install
/// would translate against has to be exactly as safe as one a project declares.
pub(crate) fn validate_layout_dir(path: &str, value: &str) -> Result<(), ContractError> {
    let invalid = value.is_empty()
        || value.len() > 1024
        || value.starts_with('/')
        || value.ends_with('/')
        || value
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || value.chars().any(|character| {
            character.is_control()
                || character == '\\'
                || matches!(character, '*' | '?' | '[' | ']')
        });
    if invalid {
        return Err(ContractError::invalid(format!(
            "{path} must be a normalized, project-relative directory"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectRweSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_list: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub minify_html: bool,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub strict_mode: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub libraries: BTreeMap<String, ProjectRweLibrarySpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deployment_asset_base: Option<String>,
}

impl Default for ProjectRweSpec {
    fn default() -> Self {
        Self {
            allow_list: Vec::new(),
            minify_html: false,
            strict_mode: true,
            libraries: BTreeMap::new(),
            deployment_asset_base: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectRweLibrarySpec {
    pub version: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectPipelinesSpec {
    #[serde(default, skip_serializing_if = "is_default")]
    pub logging: ProjectPipelineLoggingSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectPipelineLoggingSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_invocations: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRuntimeModeSpec {
    #[default]
    Shared,
    Pinned,
    Dedicated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectExecutionSpec {
    #[default]
    Resident,
    DockerJob,
    K8sJob,
    SparkSubmit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectResourceProfileSpec {
    Tiny,
    #[default]
    Small,
    Medium,
    Large,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectRuntimeSpec {
    #[serde(default, skip_serializing_if = "is_default")]
    pub mode: ProjectRuntimeModeSpec,
    #[serde(default, skip_serializing_if = "is_default")]
    pub execution: ProjectExecutionSpec,
    #[serde(default, skip_serializing_if = "is_default")]
    pub resource_profile: ProjectResourceProfileSpec,
    #[serde(default = "default_min_replicas", skip_serializing_if = "is_one")]
    pub min_replicas: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_resources: Option<ProjectRuntimeResourceSpec>,
}

impl Default for ProjectRuntimeSpec {
    fn default() -> Self {
        Self {
            mode: ProjectRuntimeModeSpec::Shared,
            execution: ProjectExecutionSpec::Resident,
            resource_profile: ProjectResourceProfileSpec::Small,
            min_replicas: 1,
            max_replicas: None,
            required_tags: Vec::new(),
            custom_resources: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectRuntimeResourceSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_request_millis: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_request_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit_millis: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral_disk_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator_count: Option<u16>,
}

impl ProjectRuntimeResourceSpec {
    fn validate(&self) -> Result<(), ContractError> {
        if [
            self.cpu_request_millis,
            self.memory_request_mb,
            self.cpu_limit_millis,
            self.memory_limit_mb,
            self.ephemeral_disk_mb,
        ]
        .into_iter()
        .flatten()
        .any(|value| value == 0)
            || self.accelerator_count.is_some_and(|value| value == 0)
        {
            return Err(ContractError::invalid(
                "spec.runtime.custom_resources values must be greater than zero",
            ));
        }
        if let (Some(request), Some(limit)) = (self.cpu_request_millis, self.cpu_limit_millis)
            && request > limit
        {
            return Err(ContractError::invalid(
                "spec.runtime.custom_resources.cpu_request_millis must not exceed cpu_limit_millis",
            ));
        }
        if let (Some(request), Some(limit)) = (self.memory_request_mb, self.memory_limit_mb)
            && request > limit
        {
            return Err(ContractError::invalid(
                "spec.runtime.custom_resources.memory_request_mb must not exceed memory_limit_mb",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBootstrapSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activate: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectGitSpec {
    #[serde(default, skip_serializing_if = "is_default")]
    pub remote: ProjectGitRemoteSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectGitRemoteSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub credential_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectAssistantSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_model_credential: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_model_credential: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_steps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replans: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_history_pairs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectLocksSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<String>,
}

/// Reserved v1 section for future data policy settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectDataSpec {}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectFilesSpec {
    #[serde(default, skip_serializing_if = "is_default")]
    pub uploads: ProjectUploadSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectUploadSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_asset_size_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_body_max_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_size_mb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectDistributionSpec {
    #[serde(default)]
    pub hub: ProjectHubDistributionSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectHubDistributionSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub entry_url: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub as_app: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub producer_enabled: bool,
}

impl From<ZebflowJson> for ProjectConfigurationSpec {
    fn from(value: ZebflowJson) -> Self {
        let default_uploads = ZebflowJsonUploads::default();
        Self {
            profile: ProjectProfileSpec {
                title: value.metadata.title,
                description: value.metadata.description,
            },
            layout: ProjectLayoutSpec {
                source: value.configs.layout.source,
                assets: value.configs.layout.assets,
                docs: value.configs.layout.docs,
                schema: value.configs.layout.schema,
                sqlite_schema: value.configs.layout.sqlite_schema,
                node_interfaces: value.configs.layout.node_interfaces,
                initial_data: value.configs.layout.initial_data.map(|entries| {
                    entries
                        .into_iter()
                        .map(|entry| ProjectInitialDataDirSpec {
                            path: entry.path,
                            engine: entry.engine,
                        })
                        .collect()
                }),
            },
            rwe: ProjectRweSpec {
                allow_list: value.configs.rwe.allow_list,
                minify_html: value.configs.rwe.minify_html,
                strict_mode: value.configs.rwe.strict_mode,
                libraries: value
                    .configs
                    .rwe
                    .libraries
                    .into_iter()
                    .map(|(name, item)| {
                        (
                            name,
                            ProjectRweLibrarySpec {
                                version: item.version,
                                source: item.source,
                            },
                        )
                    })
                    .collect(),
                deployment_asset_base: value.configs.rwe.deployment_asset_base,
            },
            pipelines: ProjectPipelinesSpec {
                logging: ProjectPipelineLoggingSpec {
                    max_invocations: value.configs.pipelines.logging.max_invocations,
                },
                node_timeout_secs: value.configs.pipelines.node_timeout_secs,
            },
            runtime: value.configs.runtime.into(),
            bootstrap: ProjectBootstrapSpec {
                activate: value.configs.bootstrap.activate,
            },
            git: ProjectGitSpec {
                remote: ProjectGitRemoteSpec {
                    credential_id: value.configs.git.remote.credential_id,
                    repo_url: value.configs.git.remote.repo_url,
                    branch: value.configs.git.remote.branch,
                },
            },
            assistant: ProjectAssistantSpec {
                high_model_credential: value.configs.assistant.high_model_credential,
                general_model_credential: value.configs.assistant.general_model_credential,
                max_steps: value.configs.assistant.max_steps,
                max_replans: value.configs.assistant.max_replans,
                chat_history_pairs: value.configs.assistant.chat_history_pairs,
                enabled: value.configs.assistant.enabled,
            },
            locks: ProjectLocksSpec {
                templates: value.configs.locks.templates,
            },
            data: ProjectDataSpec {},
            files: ProjectFilesSpec {
                uploads: ProjectUploadSpec {
                    max_asset_size_mb: (value.configs.files.uploads.max_asset_size_mb
                        != default_uploads.max_asset_size_mb)
                        .then_some(value.configs.files.uploads.max_asset_size_mb),
                    webhook_body_max_mb: (value.configs.files.uploads.webhook_body_max_mb
                        != default_uploads.webhook_body_max_mb)
                        .then_some(value.configs.files.uploads.webhook_body_max_mb),
                    max_file_size_mb: value.configs.files.uploads.max_file_size_mb,
                },
            },
            distribution: ProjectDistributionSpec {
                hub: ProjectHubDistributionSpec {
                    entry_url: value.distribution.hub.entry_url,
                    as_app: value.distribution.hub.as_app,
                    producer_enabled: value.distribution.hub.producer_enabled,
                },
            },
        }
    }
}

impl From<ProjectConfigurationSpec> for ZebflowJson {
    fn from(value: ProjectConfigurationSpec) -> Self {
        let default_uploads = ZebflowJsonUploads::default();
        Self {
            metadata: ZebflowJsonMetadata {
                title: value.profile.title,
                description: value.profile.description,
            },
            configs: ZebflowJsonConfigs {
                layout: ZebflowJsonLayout {
                    source: value.layout.source,
                    assets: value.layout.assets,
                    docs: value.layout.docs,
                    schema: value.layout.schema,
                    sqlite_schema: value.layout.sqlite_schema,
                    node_interfaces: value.layout.node_interfaces,
                    initial_data: value.layout.initial_data.map(|entries| {
                        entries
                            .into_iter()
                            .map(|entry| ZebflowJsonInitialDataDir {
                                path: entry.path,
                                engine: entry.engine,
                            })
                            .collect()
                    }),
                },
                rwe: ZebflowJsonRwe {
                    allow_list: value.rwe.allow_list,
                    minify_html: value.rwe.minify_html,
                    strict_mode: value.rwe.strict_mode,
                    libraries: value
                        .rwe
                        .libraries
                        .into_iter()
                        .map(|(name, item)| {
                            (
                                name,
                                ZebflowJsonRweLibraryEntry {
                                    version: item.version,
                                    source: item.source,
                                },
                            )
                        })
                        .collect(),
                    deployment_asset_base: value.rwe.deployment_asset_base,
                },
                pipelines: ZebflowJsonPipelines {
                    logging: ZebflowJsonLogging {
                        max_invocations: value.pipelines.logging.max_invocations,
                    },
                    node_timeout_secs: value.pipelines.node_timeout_secs,
                },
                runtime: value.runtime.into(),
                bootstrap: ProjectBootstrapPlan {
                    activate: value.bootstrap.activate,
                },
                git: ZebflowJsonGit {
                    remote: ZebflowJsonGitRemote {
                        credential_id: value.git.remote.credential_id,
                        repo_url: value.git.remote.repo_url,
                        branch: value.git.remote.branch,
                    },
                },
                assistant: ZebflowJsonAssistant {
                    high_model_credential: value.assistant.high_model_credential,
                    general_model_credential: value.assistant.general_model_credential,
                    max_steps: value.assistant.max_steps,
                    max_replans: value.assistant.max_replans,
                    chat_history_pairs: value.assistant.chat_history_pairs,
                    enabled: value.assistant.enabled,
                },
                locks: ZebflowJsonLocks {
                    templates: value.locks.templates,
                },
                data: ZebflowJsonData {},
                files: ZebflowJsonFiles {
                    uploads: ZebflowJsonUploads {
                        max_asset_size_mb: value
                            .files
                            .uploads
                            .max_asset_size_mb
                            .unwrap_or(default_uploads.max_asset_size_mb),
                        webhook_body_max_mb: value
                            .files
                            .uploads
                            .webhook_body_max_mb
                            .unwrap_or(default_uploads.webhook_body_max_mb),
                        max_file_size_mb: value.files.uploads.max_file_size_mb,
                    },
                },
            },
            distribution: ZebflowJsonDistribution {
                hub: ZebflowJsonDistributionHub {
                    entry_url: value.distribution.hub.entry_url,
                    as_app: value.distribution.hub.as_app,
                    producer_enabled: value.distribution.hub.producer_enabled,
                },
            },
        }
    }
}

impl From<ProjectRuntimeProfile> for ProjectRuntimeSpec {
    fn from(value: ProjectRuntimeProfile) -> Self {
        Self {
            mode: match value.mode {
                ProjectRuntimeMode::Shared => ProjectRuntimeModeSpec::Shared,
                ProjectRuntimeMode::Pinned => ProjectRuntimeModeSpec::Pinned,
                ProjectRuntimeMode::Dedicated => ProjectRuntimeModeSpec::Dedicated,
            },
            execution: match value.execution {
                ExecutionProfileKind::Resident => ProjectExecutionSpec::Resident,
                ExecutionProfileKind::DockerJob => ProjectExecutionSpec::DockerJob,
                ExecutionProfileKind::K8sJob => ProjectExecutionSpec::K8sJob,
                ExecutionProfileKind::SparkSubmit => ProjectExecutionSpec::SparkSubmit,
            },
            resource_profile: match value.resource_profile {
                ResourceProfile::Tiny => ProjectResourceProfileSpec::Tiny,
                ResourceProfile::Small => ProjectResourceProfileSpec::Small,
                ResourceProfile::Medium => ProjectResourceProfileSpec::Medium,
                ResourceProfile::Large => ProjectResourceProfileSpec::Large,
                ResourceProfile::Custom => ProjectResourceProfileSpec::Custom,
            },
            min_replicas: value.min_replicas,
            max_replicas: value.max_replicas,
            required_tags: value.required_tags,
            custom_resources: value.custom_resources.map(Into::into),
        }
    }
}

impl From<ProjectRuntimeSpec> for ProjectRuntimeProfile {
    fn from(value: ProjectRuntimeSpec) -> Self {
        Self {
            mode: match value.mode {
                ProjectRuntimeModeSpec::Shared => ProjectRuntimeMode::Shared,
                ProjectRuntimeModeSpec::Pinned => ProjectRuntimeMode::Pinned,
                ProjectRuntimeModeSpec::Dedicated => ProjectRuntimeMode::Dedicated,
            },
            execution: match value.execution {
                ProjectExecutionSpec::Resident => ExecutionProfileKind::Resident,
                ProjectExecutionSpec::DockerJob => ExecutionProfileKind::DockerJob,
                ProjectExecutionSpec::K8sJob => ExecutionProfileKind::K8sJob,
                ProjectExecutionSpec::SparkSubmit => ExecutionProfileKind::SparkSubmit,
            },
            resource_profile: match value.resource_profile {
                ProjectResourceProfileSpec::Tiny => ResourceProfile::Tiny,
                ProjectResourceProfileSpec::Small => ResourceProfile::Small,
                ProjectResourceProfileSpec::Medium => ResourceProfile::Medium,
                ProjectResourceProfileSpec::Large => ResourceProfile::Large,
                ProjectResourceProfileSpec::Custom => ResourceProfile::Custom,
            },
            min_replicas: value.min_replicas,
            max_replicas: value.max_replicas,
            required_tags: value.required_tags,
            custom_resources: value.custom_resources.map(Into::into),
        }
    }
}

impl From<RuntimeResourceSpec> for ProjectRuntimeResourceSpec {
    fn from(value: RuntimeResourceSpec) -> Self {
        Self {
            cpu_request_millis: value.cpu_request_millis,
            memory_request_mb: value.memory_request_mb,
            cpu_limit_millis: value.cpu_limit_millis,
            memory_limit_mb: value.memory_limit_mb,
            ephemeral_disk_mb: value.ephemeral_disk_mb,
            accelerator_count: value.accelerator_count,
        }
    }
}

impl From<ProjectRuntimeResourceSpec> for RuntimeResourceSpec {
    fn from(value: ProjectRuntimeResourceSpec) -> Self {
        Self {
            cpu_request_millis: value.cpu_request_millis,
            memory_request_mb: value.memory_request_mb,
            cpu_limit_millis: value.cpu_limit_millis,
            memory_limit_mb: value.memory_limit_mb,
            ephemeral_disk_mb: value.ephemeral_disk_mb,
            accelerator_count: value.accelerator_count,
        }
    }
}

const fn default_true() -> bool {
    true
}

const fn default_min_replicas() -> u32 {
    1
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn is_one(value: &u32) -> bool {
    *value == 1
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::contracts::{
        ContractMetadata, decode_contract_value, decode_contract_yaml, encode_contract,
        encode_contract_yaml,
    };
    use crate::platform::model::ResolvedProjectLayout;

    const COMPLETE_V1: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/project-configuration/v1-complete.yaml");
    const COMPLETE_LEGACY_V1: &[u8] = include_bytes!(
        "../../../tests/fixtures/contracts/project-configuration/legacy-1.0-complete.json"
    );
    const DECLARED_LAYOUT_V1: &[u8] =
        include_bytes!("../../../tests/fixtures/contracts/project-configuration/v1-layout.yaml");

    fn declared_layout() -> ProjectLayoutSpec {
        ProjectLayoutSpec {
            source: Some("src".to_string()),
            assets: Some("src/assets".to_string()),
            docs: Some("documentation".to_string()),
            schema: Some("db/schema".to_string()),
            sqlite_schema: Some("db/sqlite".to_string()),
            node_interfaces: Some("vendor/nodes".to_string()),
            initial_data: Some(vec![ProjectInitialDataDirSpec {
                path: "db/seeds".to_string(),
                engine: "sekejap".to_string(),
            }]),
        }
    }

    #[test]
    fn an_undeclared_layout_resolves_to_the_hardcoded_platform_directories() {
        let document = decode_contract_yaml::<ProjectConfigurationContract>(COMPLETE_V1).unwrap();
        assert_eq!(document.spec.layout, ProjectLayoutSpec::default());

        let layout = ZebflowJson::from(document.spec).layout();
        assert_eq!(layout.source, "pipelines");
        assert_eq!(layout.assets, "pipelines/assets");
        assert_eq!(layout.docs, "docs");
        assert_eq!(layout.schema, "schemas/sekejap");
        assert_eq!(layout.sqlite_schema, "schemas/sqlite");
        assert_eq!(layout.node_interfaces, "nodes");
        assert_eq!(
            layout
                .initial_data
                .iter()
                .map(|entry| (entry.path.as_str(), entry.engine.as_str()))
                .collect::<Vec<_>>(),
            [
                ("initial-data/sekejap", "sekejap"),
                ("initial-data/sqlite", "sqlite"),
                ("init/sekejap", "sekejap"),
                ("init/sqlite", "sqlite"),
                ("seeds/sekejap", "sekejap"),
                ("seeds/sqlite", "sqlite"),
            ]
        );
        assert_eq!(layout, ResolvedProjectLayout::platform_default());
    }

    #[test]
    fn declared_layout_fixture_roundtrips_byte_for_byte() {
        let document =
            decode_contract_yaml::<ProjectConfigurationContract>(DECLARED_LAYOUT_V1).unwrap();
        assert_eq!(document.spec.layout, declared_layout());
        assert_eq!(
            encode_contract_yaml::<ProjectConfigurationContract>(document.metadata, document.spec)
                .unwrap(),
            DECLARED_LAYOUT_V1
        );
    }

    #[test]
    fn a_declared_layout_survives_the_runtime_model_roundtrip() {
        let expected = ProjectConfigurationSpec {
            layout: declared_layout(),
            ..ProjectConfigurationSpec::default()
        };
        let runtime: ZebflowJson = expected.clone().into();
        assert_eq!(runtime.layout().source, "src");
        // Undeclared entries still resolve to the platform default.
        assert_eq!(runtime.layout().assets, "src/assets");
        assert_eq!(ProjectConfigurationSpec::from(runtime), expected);
    }

    #[test]
    fn unusable_layout_directories_and_engines_are_rejected() {
        let seed = |path: &str, engine: &str| ProjectInitialDataDirSpec {
            path: path.to_string(),
            engine: engine.to_string(),
        };
        for layout in [
            ProjectLayoutSpec {
                source: Some("/pipelines".to_string()),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                source: Some("pipelines/".to_string()),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                source: Some("../pipelines".to_string()),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                source: Some(String::new()),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                assets: Some("pipelines/*".to_string()),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                initial_data: Some(vec![seed("seeds/mysql", "mysql")]),
                ..ProjectLayoutSpec::default()
            },
            ProjectLayoutSpec {
                initial_data: Some(vec![
                    seed("seeds/sekejap", "sekejap"),
                    seed("seeds/sekejap", "sqlite"),
                ]),
                ..ProjectLayoutSpec::default()
            },
        ] {
            assert!(
                encode_contract::<ProjectConfigurationContract>(
                    ContractMetadata::named("project"),
                    ProjectConfigurationSpec {
                        layout,
                        ..ProjectConfigurationSpec::default()
                    },
                )
                .is_err()
            );
        }
    }

    #[test]
    fn default_document_uses_the_permanent_section_layout() {
        let encoded = encode_contract::<ProjectConfigurationContract>(
            ContractMetadata::named("project-development"),
            ProjectConfigurationSpec::default(),
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(value["apiVersion"], "zebflow.com/v1");
        assert_eq!(value["kind"], "ProjectConfiguration");
        assert_eq!(value["spec"]["data"], json!({}));
        assert_eq!(value["spec"]["distribution"]["hub"], json!({}));
        assert!(value["spec"].get("configs").is_none());
        assert!(value["spec"].get("settings").is_none());
    }

    #[test]
    fn complete_v1_fixture_is_canonical_and_roundtrips_byte_for_byte() {
        let document = decode_contract_yaml::<ProjectConfigurationContract>(COMPLETE_V1).unwrap();
        assert_eq!(document.metadata.name, "project-development");
        assert_eq!(document.spec.runtime.min_replicas, 2);
        assert_eq!(document.spec.files.uploads.webhook_body_max_mb, Some(512));
        assert_eq!(
            encode_contract_yaml::<ProjectConfigurationContract>(document.metadata, document.spec,)
                .unwrap(),
            COMPLETE_V1
        );
    }

    #[test]
    fn unknown_and_invalid_settings_are_rejected() {
        let unknown = json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "ProjectConfiguration",
            "metadata": {"name": "project"},
            "spec": {"data": {"records": []}}
        });
        assert!(decode_contract_value::<ProjectConfigurationContract>(unknown).is_err());

        let invalid = json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "ProjectConfiguration",
            "metadata": {"name": "project"},
            "spec": {"pipelines": {"node_timeout_secs": 1}}
        });
        assert!(decode_contract_value::<ProjectConfigurationContract>(invalid).is_err());
    }

    #[test]
    fn disk_and_runtime_models_roundtrip() {
        let document = decode_contract_yaml::<ProjectConfigurationContract>(COMPLETE_V1).unwrap();
        let expected = document.spec;
        let runtime: ZebflowJson = expected.clone().into();
        let restored = ProjectConfigurationSpec::from(runtime);
        assert_eq!(restored, expected);
    }

    #[test]
    fn runtime_defaults_match_frozen_contract_defaults() {
        let runtime = ZebflowJson::default();
        let runtime_spec = ProjectConfigurationSpec::from(runtime);
        assert_eq!(runtime_spec, ProjectConfigurationSpec::default());
    }

    #[test]
    fn assistant_limits_match_the_settings_interface() {
        let mut spec = ProjectConfigurationSpec::default();
        spec.assistant.max_steps = Some(1);
        spec.assistant.max_replans = Some(0);
        spec.assistant.chat_history_pairs = Some(0);
        assert!(
            encode_contract::<ProjectConfigurationContract>(
                ContractMetadata::named("project"),
                spec.clone(),
            )
            .is_ok()
        );

        spec.assistant.max_replans = Some(65);
        assert!(
            encode_contract::<ProjectConfigurationContract>(
                ContractMetadata::named("project"),
                spec,
            )
            .is_err()
        );
    }

    #[test]
    fn explicit_legacy_decoder_maps_data_and_marketplace() {
        let legacy = br#"{
            "version":"1.0",
            "metadata":{"title":"Legacy","description":""},
            "configs":{"data":{},"pipelines":{"nodes":{}}},
            "distribution":{"marketplace":{"as_app":true}}
        }"#;
        let migrated = decode_legacy_project_configuration(legacy).unwrap();
        assert_eq!(migrated.profile.title, "Legacy");
        assert!(migrated.distribution.hub.as_app);
        assert_eq!(migrated.data, ProjectDataSpec {});
    }

    #[test]
    fn complete_legacy_fixture_preserves_every_owned_section() {
        let migrated = decode_legacy_project_configuration(COMPLETE_LEGACY_V1).unwrap();
        assert_eq!(migrated.profile.title, "Legacy Project");
        assert_eq!(migrated.rwe.libraries["zeb/deckgl"].version, "bridge-0.1");
        assert_eq!(migrated.runtime.min_replicas, 2);
        assert_eq!(migrated.runtime.max_replicas, Some(4));
        assert_eq!(migrated.bootstrap.activate, ["pipelines/main.zf.json"]);
        assert_eq!(migrated.files.uploads.webhook_body_max_mb, Some(512));
        assert!(migrated.distribution.hub.as_app);
    }

    #[test]
    fn project_metadata_is_identity_only() {
        for metadata in [
            ContractMetadata {
                version: Some("1.0.0".to_string()),
                ..ContractMetadata::named("project")
            },
            ContractMetadata {
                digest: Some(format!("sha256:{}", "0".repeat(64))),
                ..ContractMetadata::named("project")
            },
            ContractMetadata {
                annotations: [("owner".to_string(), "team".to_string())]
                    .into_iter()
                    .collect(),
                ..ContractMetadata::named("project")
            },
        ] {
            assert!(
                encode_contract::<ProjectConfigurationContract>(
                    metadata,
                    ProjectConfigurationSpec::default()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn secret_bearing_urls_and_ambiguous_lists_are_rejected() {
        let mut spec = ProjectConfigurationSpec::default();
        spec.git.remote.repo_url = "https://token@git.example.com/project.git".to_string();
        spec.git.remote.branch = "main".to_string();
        assert!(
            encode_contract::<ProjectConfigurationContract>(
                ContractMetadata::named("project"),
                spec
            )
            .is_err()
        );

        let mut spec = ProjectConfigurationSpec::default();
        spec.rwe.allow_list = vec![
            "https://cdn.example.com/*".to_string(),
            "https://cdn.example.com/*".to_string(),
        ];
        assert!(
            encode_contract::<ProjectConfigurationContract>(
                ContractMetadata::named("project"),
                spec
            )
            .is_err()
        );

        let mut spec = ProjectConfigurationSpec::default();
        spec.rwe.libraries.insert(
            "zeb/deckgl".to_string(),
            ProjectRweLibrarySpec {
                version: " ".to_string(),
                source: "offline".to_string(),
            },
        );
        assert!(
            encode_contract::<ProjectConfigurationContract>(
                ContractMetadata::named("project"),
                spec
            )
            .is_err()
        );
    }
}
