//! This instance's own settings: `<data-root>/platform/settings.json`
//! (`instance-directory.md`), read and written through
//! `/api/platform/settings` and the `/settings` page.
//!
//! They are what the office itself allows, as opposed to what a project
//! declares in its `zebflow.yaml`. Each office has its own, because each has
//! its own memory: a share written as `"auto"` is taken from the memory limit
//! this process actually runs under — its container's when it has one, the
//! machine's otherwise. A missing file is every setting at its default.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;

/// The file, under the data root.
const SETTINGS_FILE: &str = "platform/settings.json";
/// A share left to the instance.
const AUTO: &str = "auto";
/// The JavaScript heaps' share of memory when `"auto"`: a quarter, as the
/// JVM's own default heap is. Most of a run's work is in Rust; the rest is
/// left to it and to the system.
const JAVASCRIPT_SHARE: u64 = 4;
/// The waiting run records' share of memory when `"auto"`: one hundredth.
const RECORD_QUEUE_SHARE: u64 = 100;
/// The share every SQL query over a file holds together: a quarter, the pool
/// of `infra::table_engine`. Not a setting: a query that needs more spills to
/// disk or fails on its own, and the process stays up.
const TABLE_QUERY_SHARE: u64 = 4;
/// The least heap a runtime is made with. The heaviest Studio page in the
/// browser suite renders within 24 MiB on a fresh runtime and not within
/// 16 MiB; this leaves it a third more. Neither `"auto"` on a small device
/// nor a size written by hand goes under it.
const MIN_RUNTIME_HEAP: u64 = 32 << 20;
/// The least the run-record queue may hold: a few large failure traces.
const MIN_RECORD_QUEUE: u64 = 1 << 20;

const INVALID: &str = "PLATFORM_SETTINGS_INVALID";

/// The settings as the file holds them. Every field has a default, so a file
/// that names one setting leaves the others at theirs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceSettings {
    #[serde(default)]
    pub javascript: JavascriptSettings,
    #[serde(default)]
    pub runs: RunSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavascriptSettings {
    /// What every JavaScript runtime's heap together may reach: `"auto"` (a
    /// quarter of the memory limit) or a size such as `"512MiB"`. It is
    /// shared evenly by the script sandbox's and the page renderer's
    /// runtimes.
    #[serde(default = "auto")]
    pub heap: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSettings {
    /// What the run records waiting to be written may hold together: `"auto"`
    /// (a hundredth of the memory limit) or a size such as `"16MB"`.
    #[serde(default = "auto")]
    pub record_queue: String,
}

impl Default for JavascriptSettings {
    fn default() -> Self {
        Self { heap: auto() }
    }
}

impl Default for RunSettings {
    fn default() -> Self {
        Self { record_queue: auto() }
    }
}

fn auto() -> String {
    AUTO.to_string()
}

/// The memory this process runs under and where that number came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct MemoryLimit {
    pub bytes: u64,
    /// `container` when a container's limit is below the machine's memory,
    /// `machine` otherwise.
    pub source: &'static str,
}

/// What the settings come to on this instance, in bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EffectiveSettings {
    pub memory: MemoryLimit,
    /// Every JavaScript heap together.
    pub javascript_heap: u64,
    /// How many runtimes share it: the script sandbox's and the page
    /// renderer's.
    pub javascript_runtimes: usize,
    /// What each one may reach.
    pub javascript_heap_per_runtime: u64,
    pub record_queue: u64,
}

/// Reads, checks, stores and applies this instance's settings.
pub struct InstanceSettingsService {
    path: PathBuf,
    data: Arc<dyn DataAdapter>,
    current: RwLock<InstanceSettings>,
}

impl InstanceSettingsService {
    /// Reads the file under `data_root`. A file that does not read as
    /// settings is refused rather than replaced with defaults: an office
    /// silently running without the limits it was given is the failure
    /// these settings exist to prevent.
    pub fn open(data_root: &std::path::Path, data: Arc<dyn DataAdapter>) -> Result<Self, PlatformError> {
        let path = data_root.join(SETTINGS_FILE);
        let settings = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<InstanceSettings>(&bytes).map_err(|err| {
                PlatformError::new(INVALID, format!("{SETTINGS_FILE} does not read as settings: {err}"))
            })?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => InstanceSettings::default(),
            Err(err) => return Err(err.into()),
        };
        effective(&settings, memory_limit())?;
        Ok(Self { path, data, current: RwLock::new(settings) })
    }

    pub fn get(&self) -> InstanceSettings {
        self.current.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// What the current settings come to here.
    pub fn effective(&self) -> Result<EffectiveSettings, PlatformError> {
        effective(&self.get(), memory_limit())
    }

    /// Puts the current settings in force: runtimes made from now on, and
    /// runtimes already running before their next run, take the new heap;
    /// records waiting stay and new ones meet the new limit.
    pub fn apply(&self) -> Result<EffectiveSettings, PlatformError> {
        let effective = self.effective()?;
        crate::language::set_runtime_heap_limit(effective.javascript_heap_per_runtime as usize);
        self.data.set_run_record_queue_limit(effective.record_queue as usize);
        Ok(effective)
    }

    /// Checks, writes and applies `settings`. Nothing is written when they
    /// do not resolve on this instance.
    pub fn set(&self, settings: InstanceSettings) -> Result<EffectiveSettings, PlatformError> {
        effective(&settings, memory_limit())?;
        let bytes = serde_json::to_vec_pretty(&settings)?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::infra::io::durable::atomic_write(&self.path, &bytes)?;
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = settings;
        self.apply()
    }
}

/// The memory limit this process runs under: its container's (cgroup) when
/// that is below the machine's memory, the machine's otherwise. V8 and the
/// allocator see only the machine's, which is why this is read here.
pub fn memory_limit() -> MemoryLimit {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let machine = system.total_memory();
    match system.cgroup_limits().map(|limits| limits.total_memory) {
        Some(container) if container > 0 && container < machine => {
            MemoryLimit { bytes: container, source: "container" }
        }
        _ => MemoryLimit { bytes: machine, source: "machine" },
    }
}

/// What every SQL query over a file may hold together: a quarter of the
/// memory limit.
pub fn table_query_memory() -> u64 {
    memory_limit().bytes / TABLE_QUERY_SHARE
}

fn effective(settings: &InstanceSettings, memory: MemoryLimit) -> Result<EffectiveSettings, PlatformError> {
    let javascript_runtimes = crate::language::sandbox_runtime_count() + crate::rwe::core::deno_worker::render_runtime_count();
    let floor = MIN_RUNTIME_HEAP * javascript_runtimes as u64;
    let javascript_heap = match share(&settings.javascript.heap, "javascript.heap", memory, JAVASCRIPT_SHARE)? {
        Share::Auto(bytes) => bytes.max(floor),
        Share::Written(bytes) if bytes < floor => {
            return Err(PlatformError::new(
                INVALID,
                format!(
                    "javascript.heap {} leaves each of {javascript_runtimes} runtimes under {}MiB; it is at least {}MiB here",
                    settings.javascript.heap,
                    MIN_RUNTIME_HEAP >> 20,
                    floor.div_ceil(1 << 20)
                ),
            ));
        }
        Share::Written(bytes) => bytes,
    };
    let record_queue = match share(&settings.runs.record_queue, "runs.record_queue", memory, RECORD_QUEUE_SHARE)? {
        Share::Auto(bytes) => bytes.max(MIN_RECORD_QUEUE),
        Share::Written(bytes) if bytes < MIN_RECORD_QUEUE => {
            return Err(PlatformError::new(
                INVALID,
                format!("runs.record_queue {} is under {}MiB", settings.runs.record_queue, MIN_RECORD_QUEUE >> 20),
            ));
        }
        Share::Written(bytes) => bytes,
    };
    // What was written by hand is checked against the memory; `"auto"` is
    // what this device has, and an instance is not refused for being small.
    let written = !is_auto(&settings.javascript.heap) || !is_auto(&settings.runs.record_queue);
    if written && javascript_heap + record_queue >= memory.bytes {
        return Err(PlatformError::new(
            INVALID,
            format!(
                "javascript.heap and runs.record_queue together reach this instance's whole {}MiB",
                memory.bytes >> 20
            ),
        ));
    }
    Ok(EffectiveSettings {
        memory,
        javascript_heap,
        javascript_runtimes,
        javascript_heap_per_runtime: javascript_heap / javascript_runtimes as u64,
        record_queue,
    })
}

enum Share {
    Auto(u64),
    Written(u64),
}

fn is_auto(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case(AUTO)
}

/// `"auto"` as `memory / divisor`, or a size with its unit.
fn share(value: &str, field: &str, memory: MemoryLimit, divisor: u64) -> Result<Share, PlatformError> {
    let value = value.trim();
    if is_auto(value) {
        return Ok(Share::Auto(memory.bytes / divisor));
    }
    crate::pipeline::nodes::shared::units::size(value, field, INVALID)
        .map(Share::Written)
        .map_err(|err| PlatformError::new(INVALID, err.message))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;
    const ONE_GIB: MemoryLimit = MemoryLimit { bytes: GIB, source: "container" };

    fn with(heap: &str, record_queue: &str) -> InstanceSettings {
        InstanceSettings {
            javascript: JavascriptSettings { heap: heap.into() },
            runs: RunSettings { record_queue: record_queue.into() },
        }
    }

    #[test]
    fn auto_is_a_quarter_for_javascript_and_a_hundredth_for_waiting_records() {
        let got = effective(&InstanceSettings::default(), ONE_GIB).expect("effective");
        // The floor depends on this machine's cores, so it is part of the answer.
        let floor = MIN_RUNTIME_HEAP * got.javascript_runtimes as u64;
        assert_eq!(got.javascript_heap, (GIB / 4).max(floor));
        assert_eq!(got.record_queue, GIB / 100);
        assert_eq!(got.javascript_heap_per_runtime, got.javascript_heap / got.javascript_runtimes as u64);
    }

    #[test]
    fn a_written_size_is_taken_as_written() {
        let written = (MIN_RUNTIME_HEAP >> 20) * 16;
        let got = effective(&with(&format!("{written}MiB"), "16MB"), ONE_GIB).expect("effective");
        assert_eq!(got.javascript_heap, written << 20);
        assert_eq!(got.record_queue, 16_000_000);
    }

    #[test]
    fn auto_on_a_small_device_still_gives_each_runtime_its_floor() {
        let small = MemoryLimit { bytes: 64 << 20, source: "container" };
        let got = effective(&InstanceSettings::default(), small).expect("effective");
        assert_eq!(got.javascript_heap_per_runtime, MIN_RUNTIME_HEAP);
        assert_eq!(got.record_queue, MIN_RECORD_QUEUE);
    }

    #[test]
    fn what_would_not_leave_a_runtime_its_floor_or_the_instance_its_memory_is_refused() {
        for (heap, queue) in [("1MiB", AUTO), (AUTO, "10KB"), ("1GiB", AUTO), ("lots", AUTO), ("512", AUTO)] {
            let err = effective(&with(heap, queue), ONE_GIB).expect_err(heap);
            assert_eq!(err.code, INVALID, "{heap} {queue}");
        }
    }

    #[test]
    fn a_file_naming_one_setting_leaves_the_others_at_their_default() {
        let settings: InstanceSettings = serde_json::from_str(r#"{ "runs": { "record_queue": "8MiB" } }"#).expect("parse");
        assert_eq!(settings.javascript.heap, AUTO);
        assert_eq!(settings.runs.record_queue, "8MiB");
        assert!(serde_json::from_str::<InstanceSettings>(r#"{ "javascript": { "heep": "auto" } }"#).is_err());
    }
}
