//! In-memory pipeline execution hit/error counters for operator visibility.

use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

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

#[derive(Debug, Clone)]
struct PipelineHitStatsRow {
    owner: String,
    project: String,
    file_rel_path: String,
    success_count: u64,
    failed_count: u64,
    last_success_at: Option<i64>,
    last_failure_at: Option<i64>,
    latest_errors: VecDeque<PipelineErrorEvent>,
}

impl PipelineHitStatsRow {
    fn new(owner: &str, project: &str, file_rel_path: &str) -> Self {
        Self {
            owner: crate::platform::model::slug_segment(owner),
            project: crate::platform::model::slug_segment(project),
            file_rel_path: file_rel_path.trim().replace('\\', "/"),
            success_count: 0,
            failed_count: 0,
            last_success_at: None,
            last_failure_at: None,
            latest_errors: VecDeque::new(),
        }
    }

    fn into_public(self) -> PipelineHitStats {
        PipelineHitStats {
            owner: self.owner,
            project: self.project,
            file_rel_path: self.file_rel_path,
            success_count: self.success_count,
            failed_count: self.failed_count,
            last_success_at: self.last_success_at,
            last_failure_at: self.last_failure_at,
            latest_errors: self.latest_errors.into_iter().collect(),
        }
    }
}

/// Lightweight runtime-only hit/error stats store.
pub struct PipelineHitsService {
    rows: RwLock<HashMap<String, PipelineHitStatsRow>>,
    latest_error_limit: usize,
}

impl PipelineHitsService {
    pub fn new(latest_error_limit: usize) -> Self {
        Self {
            rows: RwLock::new(HashMap::new()),
            latest_error_limit: latest_error_limit.max(1),
        }
    }

    pub fn record_success(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key = hit_key(owner, project, file_rel_path);
        let mut rows = self.rows.write().expect("pipeline hits write lock");
        let row = rows
            .entry(key)
            .or_insert_with(|| PipelineHitStatsRow::new(owner, project, file_rel_path));
        row.success_count = row.success_count.saturating_add(1);
        row.last_success_at = Some(now_ts());
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
        let key = hit_key(owner, project, file_rel_path);
        let mut rows = self.rows.write().expect("pipeline hits write lock");
        let row = rows
            .entry(key)
            .or_insert_with(|| PipelineHitStatsRow::new(owner, project, file_rel_path));
        row.failed_count = row.failed_count.saturating_add(1);
        row.last_failure_at = Some(now_ts());
        row.latest_errors.push_front(PipelineErrorEvent {
            at: now_ts(),
            source: source.trim().to_string(),
            code: code.trim().to_string(),
            message: message.trim().to_string(),
        });
        while row.latest_errors.len() > self.latest_error_limit {
            row.latest_errors.pop_back();
        }
    }

    pub fn get(&self, owner: &str, project: &str, file_rel_path: &str) -> PipelineHitStats {
        let key = hit_key(owner, project, file_rel_path);
        let rows = self.rows.read().expect("pipeline hits read lock");
        rows.get(&key)
            .cloned()
            .unwrap_or_else(|| PipelineHitStatsRow::new(owner, project, file_rel_path))
            .into_public()
    }

    pub fn list_project(&self, owner: &str, project: &str) -> Vec<PipelineHitStats> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        let rows = self.rows.read().expect("pipeline hits read lock");
        let mut items = rows
            .values()
            .filter(|row| row.owner == owner && row.project == project)
            .cloned()
            .map(PipelineHitStatsRow::into_public)
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
