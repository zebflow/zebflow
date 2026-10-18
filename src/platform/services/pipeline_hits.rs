//! In-memory pipeline execution hit/error counters for operator visibility.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineErrorEvent {
    pub at: i64,
    pub source: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PipelineHitStats {
    pub owner: String,
    pub project: String,
    pub file_rel_path: String,
    pub success_count: u64,
    pub failed_count: u64,
    pub last_success_at: Option<i64>,
    pub last_failure_at: Option<i64>,
    pub latest_errors: Vec<PipelineErrorEvent>,
}

/// One pipeline's counters. A success is counted with atomics under the
/// map's read lock: every run counts one, and a write lock taken per run
/// made every core wait on every other. A failure, rare and carrying its
/// recent errors, takes the row's own lock.
#[derive(Debug)]
struct PipelineHitStatsRow {
    owner: String,
    project: String,
    file_rel_path: String,
    success_count: AtomicU64,
    /// Seconds; 0 is none yet.
    last_success_at: AtomicI64,
    failures: Mutex<Failures>,
}

#[derive(Debug, Default)]
struct Failures {
    failed_count: u64,
    last_failure_at: Option<i64>,
    latest_errors: VecDeque<PipelineErrorEvent>,
}

impl PipelineHitStatsRow {
    fn new(owner: &str, project: &str, file_rel_path: &str) -> Self {
        Self {
            owner: crate::platform::model::slug_segment(owner),
            project: crate::platform::model::slug_segment(project),
            file_rel_path: file_rel_path.trim().replace('\\', "/"),
            success_count: AtomicU64::new(0),
            last_success_at: AtomicI64::new(0),
            failures: Mutex::new(Failures::default()),
        }
    }

    fn to_public(&self) -> PipelineHitStats {
        let failures = self.failures.lock().unwrap_or_else(|e| e.into_inner());
        let last_success_at = self.last_success_at.load(Ordering::Relaxed);
        PipelineHitStats {
            owner: self.owner.clone(),
            project: self.project.clone(),
            file_rel_path: self.file_rel_path.clone(),
            success_count: self.success_count.load(Ordering::Relaxed),
            failed_count: failures.failed_count,
            last_success_at: (last_success_at != 0).then_some(last_success_at),
            last_failure_at: failures.last_failure_at,
            latest_errors: failures.latest_errors.iter().cloned().collect(),
        }
    }
}

/// Lightweight runtime-only hit/error stats store.
pub struct PipelineHitsService {
    rows: RwLock<HashMap<String, Arc<PipelineHitStatsRow>>>,
    latest_error_limit: usize,
}

impl PipelineHitsService {
    pub fn new(latest_error_limit: usize) -> Self {
        Self {
            rows: RwLock::new(HashMap::new()),
            latest_error_limit: latest_error_limit.max(1),
        }
    }

    /// The row for one pipeline, made the first time it is counted.
    fn row(&self, owner: &str, project: &str, file_rel_path: &str) -> Arc<PipelineHitStatsRow> {
        let key = hit_key(owner, project, file_rel_path);
        if let Some(row) = self.rows.read().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return row.clone();
        }
        self.rows
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .entry(key)
            .or_insert_with(|| Arc::new(PipelineHitStatsRow::new(owner, project, file_rel_path)))
            .clone()
    }

    pub fn record_success(&self, owner: &str, project: &str, file_rel_path: &str) {
        let row = self.row(owner, project, file_rel_path);
        row.success_count.fetch_add(1, Ordering::Relaxed);
        row.last_success_at.store(now_ts(), Ordering::Relaxed);
    }

    pub fn record_failure(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        source: &str,
        code: &str,
        message: &str,
    ) {
        let row = self.row(owner, project, file_rel_path);
        let mut failures = row.failures.lock().unwrap_or_else(|e| e.into_inner());
        failures.failed_count = failures.failed_count.saturating_add(1);
        failures.last_failure_at = Some(now_ts());
        failures.latest_errors.push_front(PipelineErrorEvent {
            at: now_ts(),
            source: source.trim().to_string(),
            code: code.trim().to_string(),
            message: message.trim().to_string(),
        });
        while failures.latest_errors.len() > self.latest_error_limit {
            failures.latest_errors.pop_back();
        }
    }

    pub fn get(&self, owner: &str, project: &str, file_rel_path: &str) -> PipelineHitStats {
        let key = hit_key(owner, project, file_rel_path);
        let rows = self.rows.read().unwrap_or_else(|e| e.into_inner());
        match rows.get(&key) {
            Some(row) => row.to_public(),
            None => PipelineHitStatsRow::new(owner, project, file_rel_path).to_public(),
        }
    }

    pub fn list_project(&self, owner: &str, project: &str) -> Vec<PipelineHitStats> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        let rows = self.rows.read().unwrap_or_else(|e| e.into_inner());
        let mut items = rows
            .values()
            .filter(|row| row.owner == owner && row.project == project)
            .map(|row| row.to_public())
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
        items
    }
}

fn hit_key(owner: &str, project: &str, file_rel_path: &str) -> String {
    format!(
        "{}/{}/{}",
        crate::platform::model::slug_segment(owner),
        crate::platform::model::slug_segment(project),
        file_rel_path.trim().replace('\\', "/")
    )
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
