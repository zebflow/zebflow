//! Office registry orchestration service.
//!
//! This service is the product-facing owner of office inventory. It persists runtime-host records
//! in the catalog so cluster-aware UI, placement decisions, and runtime dispatch can all resolve
//! the same source of truth.

use std::sync::Arc;

use crate::infra::cluster::registry::{
    WorkerHeartbeat, WorkerRegistryRecord, WorkerRegistrySnapshot,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ClusterWorkerHeartbeatRequest, ClusterWorkerHeartbeatResponse, ClusterWorkerRegisterRequest,
    now_ts,
};

/// Product-facing office registry service.
#[derive(Clone)]
pub struct ClusterRegistryService {
    data: Arc<dyn DataAdapter>,
}

impl ClusterRegistryService {
    /// Build a registry service backed by the platform catalog adapter.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Register or refresh one office/runtime host in the catalog.
    pub fn register_worker(
        &self,
        request: &ClusterWorkerRegisterRequest,
    ) -> Result<WorkerRegistryRecord, PlatformError> {
        let now = now_ts();
        let existing = self.data.get_worker_registry_record(&request.node_id)?;
        let record = WorkerRegistryRecord {
            node_id: request.node_id.trim().to_string(),
            label: if request.label.trim().is_empty() {
                request.node_id.trim().to_string()
            } else {
                request.label.trim().to_string()
            },
            base_url: request.base_url.trim_end_matches('/').to_string(),
            status: existing
                .as_ref()
                .map(|value| value.status.clone())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "online".to_string()),
            capabilities: request.capabilities.clone(),
            registered_at: existing
                .as_ref()
                .map(|value| value.registered_at)
                .unwrap_or(now),
            last_heartbeat_at: now,
        };
        self.data.put_worker_registry_record(&record)?;
        Ok(record)
    }

    /// Persist one office heartbeat update.
    pub fn heartbeat(
        &self,
        request: &ClusterWorkerHeartbeatRequest,
    ) -> Result<ClusterWorkerHeartbeatResponse, PlatformError> {
        let now = now_ts();
        let existing = self
            .data
            .get_worker_registry_record(&request.node_id)?
            .ok_or_else(|| {
                PlatformError::new(
                    "CLUSTER_WORKER_UNKNOWN",
                    format!("office '{}' is not registered", request.node_id),
                )
            })?;
        let mut record = existing.clone();
        if !request.status.trim().is_empty() {
            record.status = request.status.trim().to_string();
        }
        if !request.base_url.trim().is_empty() {
            record.base_url = request.base_url.trim_end_matches('/').to_string();
        }
        if !request.capabilities.tags.is_empty()
            || request.capabilities.supports_resident
            || request.capabilities.supports_k8s_job
            || request.capabilities.supports_spark_submit
        {
            record.capabilities = request.capabilities.clone();
        }
        record.last_heartbeat_at = now;
        self.data.put_worker_registry_record(&record)?;
        Ok(ClusterWorkerHeartbeatResponse {
            ok: true,
            heartbeat: WorkerHeartbeat {
                node_id: record.node_id,
                at: now,
                status: record.status,
                base_url: record.base_url,
                capabilities: record.capabilities,
            },
        })
    }

    /// Return one office/runtime host by id.
    pub fn get_worker(&self, node_id: &str) -> Result<Option<WorkerRegistryRecord>, PlatformError> {
        self.data.get_worker_registry_record(node_id)
    }

    /// Return the full office snapshot.
    pub fn snapshot(&self) -> Result<WorkerRegistrySnapshot, PlatformError> {
        let workers = self.data.list_worker_registry_records()?;
        Ok(WorkerRegistrySnapshot { workers })
    }

    /// Build runtime target options for project create/clone UI.
    pub fn runtime_target_options(
        &self,
    ) -> Result<Vec<crate::platform::model::ClusterRuntimeTargetOption>, PlatformError> {
        let mut options = vec![crate::platform::model::ClusterRuntimeTargetOption {
            value: "local".to_string(),
            label: "Local office".to_string(),
            description: "Run inside the current self-controlled office.".to_string(),
        }];
        for worker in self.data.list_worker_registry_records()? {
            options.push(crate::platform::model::ClusterRuntimeTargetOption {
                value: worker.node_id.clone(),
                label: worker.label.clone(),
                description: if worker.base_url.is_empty() {
                    worker.status.clone()
                } else {
                    format!("{} · {}", worker.base_url, worker.status)
                },
            });
        }
        Ok(options)
    }
}
