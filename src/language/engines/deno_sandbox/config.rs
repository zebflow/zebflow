//! Deno sandbox configuration model and patch-merging helpers.

use serde::{Deserialize, Serialize};

/// Concrete runtime configuration after patch resolution.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxConfig {
    /// In-run wall clock budget checked by the runner.
    pub timeout_ms: u64,
    /// Host watchdog budget for killing the Deno subprocess.
    pub host_kill_timeout_ms: u64,
    /// Maximum operation ticks before forced failure.
    pub max_ops: u64,
    /// Maximum accepted source size (bytes) at compile stage.
    pub max_source_bytes: usize,
    /// Maximum allowed stdout payload returned from runner (bytes).
    pub max_output_bytes: usize,
    /// Root path used for local fetches such as `/data/file.json`.
    pub local_fetch_root: String,
    /// Allow-list configuration for fetch targets.
    pub allow_list: DenoSandboxAllowList,
    /// Capability names exposed into script as `n.*`.
    pub capabilities: Vec<String>,
    /// Dangerous toggles that must stay closed by default.
    pub danger_zone: DenoSandboxDangerZone,
}

impl Default for DenoSandboxConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 100,
            host_kill_timeout_ms: 1_500,
            max_ops: 1_000_000,
            max_source_bytes: 128 * 1024,
            max_output_bytes: 64 * 1024,
            local_fetch_root: ".".into(),
            allow_list: DenoSandboxAllowList::default(),
            capabilities: vec!["time.now".into(), "math.imul".into(), "math.u32".into()],
            danger_zone: DenoSandboxDangerZone::default(),
        }
    }
}

/// Safe allow-list options for network and resource access.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxAllowList {
    /// Explicit external hosts (or host:port) allowed by `fetch`.
    pub external_fetch_hosts: Vec<String>,
}

/// Explicit dangerous permissions, all denied by default.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxDangerZone {
    /// Allows eval/new Function and string-based codegen.
    pub allow_dynamic_code: bool,
    /// Allows import statements and dynamic import.
    pub allow_import: bool,
    /// Allows timer APIs.
    pub allow_timers: bool,
    /// Allows network access beyond explicit allow-list.
    pub allow_net: bool,
    /// Host filter used with `allow_net`.
    pub allow_net_hosts: Vec<String>,
    /// Additional read paths.
    pub allow_read_paths: Vec<String>,
    /// Additional write paths.
    pub allow_write_paths: Vec<String>,
    /// Allowed env keys.
    pub allow_env_keys: Vec<String>,
    /// Allows subprocess execution.
    pub allow_run: bool,
    /// Disables loop guard injection.
    pub disable_loop_guards: bool,
    /// Removes all runtime clamps.
    pub allow_unbounded_limits: bool,
}

/// Partial patch used in platform/project/run layering.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxConfigPatch {
    /// Optional wall-clock timeout override.
    pub timeout_ms: Option<u64>,
    /// Optional host watchdog timeout override.
    pub host_kill_timeout_ms: Option<u64>,
    /// Optional op-budget override.
    pub max_ops: Option<u64>,
    /// Optional source-size limit override.
    pub max_source_bytes: Option<usize>,
    /// Optional output-size limit override.
    pub max_output_bytes: Option<usize>,
    /// Optional local fetch root override.
    pub local_fetch_root: Option<String>,
    /// Optional allow-list patch.
    pub allow_list: Option<DenoSandboxAllowListPatch>,
    /// Optional capability override.
    pub capabilities: Option<Vec<String>>,
    /// Optional dangerous-flags patch.
    pub danger_zone: Option<DenoSandboxDangerZonePatch>,
}

/// Partial allow-list patch.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxAllowListPatch {
    /// Optional replacement for `external_fetch_hosts`.
    pub external_fetch_hosts: Option<Vec<String>>,
}

/// Partial dangerous-options patch.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoSandboxDangerZonePatch {
    /// Optional override for dynamic code generation allowance.
    pub allow_dynamic_code: Option<bool>,
    /// Optional override for import allowance.
    pub allow_import: Option<bool>,
    /// Optional override for timers allowance.
    pub allow_timers: Option<bool>,
    /// Optional override for unrestricted network allowance.
    pub allow_net: Option<bool>,
    /// Optional host filter override used with `allow_net`.
    pub allow_net_hosts: Option<Vec<String>>,
    /// Optional extra read paths override.
    pub allow_read_paths: Option<Vec<String>>,
    /// Optional extra write paths override.
    pub allow_write_paths: Option<Vec<String>>,
    /// Optional allowed env keys override.
    pub allow_env_keys: Option<Vec<String>>,
    /// Optional subprocess execution override.
    pub allow_run: Option<bool>,
    /// Optional loop guard instrumentation override.
    pub disable_loop_guards: Option<bool>,
    /// Optional unlimited limits override.
    pub allow_unbounded_limits: Option<bool>,
}

/// Applies a patch into a mutable config.
pub(crate) fn apply_patch(cfg: &mut DenoSandboxConfig, patch: &DenoSandboxConfigPatch) {
    if let Some(v) = patch.timeout_ms {
        cfg.timeout_ms = v;
    }
    if let Some(v) = patch.host_kill_timeout_ms {
        cfg.host_kill_timeout_ms = v;
    }
    if let Some(v) = patch.max_ops {
        cfg.max_ops = v;
    }
    if let Some(v) = patch.max_source_bytes {
        cfg.max_source_bytes = v;
    }
    if let Some(v) = patch.max_output_bytes {
        cfg.max_output_bytes = v;
    }
    if let Some(v) = &patch.local_fetch_root {
        cfg.local_fetch_root = v.clone();
    }
    if let Some(list) = &patch.allow_list
        && let Some(v) = &list.external_fetch_hosts
    {
        cfg.allow_list.external_fetch_hosts = v.clone();
    }
    if let Some(v) = &patch.capabilities {
        cfg.capabilities = v.clone();
    }
    if let Some(dz) = &patch.danger_zone {
        if let Some(v) = dz.allow_dynamic_code {
            cfg.danger_zone.allow_dynamic_code = v;
        }
        if let Some(v) = dz.allow_import {
            cfg.danger_zone.allow_import = v;
        }
        if let Some(v) = dz.allow_timers {
            cfg.danger_zone.allow_timers = v;
        }
        if let Some(v) = dz.allow_net {
            cfg.danger_zone.allow_net = v;
        }
        if let Some(v) = &dz.allow_net_hosts {
            cfg.danger_zone.allow_net_hosts = v.clone();
        }
        if let Some(v) = &dz.allow_read_paths {
            cfg.danger_zone.allow_read_paths = v.clone();
        }
        if let Some(v) = &dz.allow_write_paths {
            cfg.danger_zone.allow_write_paths = v.clone();
        }
        if let Some(v) = &dz.allow_env_keys {
            cfg.danger_zone.allow_env_keys = v.clone();
        }
        if let Some(v) = dz.allow_run {
            cfg.danger_zone.allow_run = v;
        }
        if let Some(v) = dz.disable_loop_guards {
            cfg.danger_zone.disable_loop_guards = v;
        }
        if let Some(v) = dz.allow_unbounded_limits {
            cfg.danger_zone.allow_unbounded_limits = v;
        }
    }
}

/// Normalizes and clamps limits to keep defaults strict and predictable.
pub(crate) fn normalize_limits(cfg: &mut DenoSandboxConfig) {
    if !cfg.danger_zone.allow_unbounded_limits {
        cfg.timeout_ms = cfg.timeout_ms.clamp(5, 60_000);
        cfg.host_kill_timeout_ms = cfg.host_kill_timeout_ms.clamp(10, 120_000);
        cfg.max_ops = cfg.max_ops.clamp(1_000, 50_000_000);
        cfg.max_source_bytes = cfg.max_source_bytes.clamp(1_024, 1_024 * 1_024);
        cfg.max_output_bytes = cfg.max_output_bytes.clamp(256, 1_024 * 1_024);
    }
    if cfg.host_kill_timeout_ms < cfg.timeout_ms + 5 {
        cfg.host_kill_timeout_ms = cfg.timeout_ms + 5;
    }
    if cfg.local_fetch_root.trim().is_empty() {
        cfg.local_fetch_root = ".".into();
    }
    let mut hosts: Vec<String> = cfg
        .allow_list
        .external_fetch_hosts
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .filter(|h| !h.is_empty())
        .collect();
    hosts.sort();
    hosts.dedup();
    cfg.allow_list.external_fetch_hosts = hosts;
}
