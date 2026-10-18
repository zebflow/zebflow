//! What a room runs: the pipelines resolved for it, and one run of one.
//!
//! The pipelines whose `trigger.room` matches a room are resolved when its
//! loop starts and again when the project's active set changes
//! ([`RoomHandle::epoch`]) — never per message (`rooms.md` § The fast lane).
//! A run counts into the room's counters and writes no run record; a failed
//! one is recorded at most once a minute per pipeline.

use std::sync::Arc;

use super::*;
use crate::infra::transport::ws::{Counter, RoomEvent, RoomHandle};
use crate::platform::services::pipeline_runtime::CompiledPipeline;

/// What every run of one room needs.
#[derive(Clone)]
pub(super) struct RoomCtx {
    pub owner: String,
    pub project: String,
    pub room_id: String,
    pub state: PlatformAppState,
}

/// The pipelines that match one room, and the engine that runs them.
pub(super) struct Resolved {
    epoch: u64,
    /// Sorted by path, so a room runs them in the same order every time.
    pipelines: Vec<Arc<CompiledPipeline>>,
    engine: crate::pipeline::BasicPipelineEngine,
}

impl Resolved {
    /// Resolve the room's pipelines from the project's active set now.
    pub(super) fn of(ctx: &RoomCtx, room: &RoomHandle) -> Self {
        // Read the epoch first: a change after it raises it again, and the
        // loop resolves once more.
        let epoch = room.epoch();
        let mut pipelines: Vec<Arc<CompiledPipeline>> = ctx
            .state
            .platform
            .pipeline_runtime
            .list_project(&ctx.owner, &ctx.project)
            .iter()
            .filter(|compiled| compiled.ws_triggers.iter().any(|t| admission::declares(&t.room, &ctx.room_id)))
            .cloned()
            .collect();
        pipelines.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
        let state = &ctx.state;
        let engine = crate::pipeline::BasicPipelineEngine::new(
            Arc::new(state.platform.project_sandbox(&ctx.owner, &ctx.project)),
            state.frontend.rwe.clone(),
            Some(state.platform.credentials.clone()),
        )
        .with_platform(state.platform.clone())
        .with_ws_hub(state.platform.ws_hub.clone())
        .with_ws_client_manager(state.ws_client_manager.clone())
        .with_state_bus(state.platform.state_bus.clone())
        .with_data_root(state.platform.config.data_root.clone());
        Self { epoch, pipelines, engine }
    }

    /// Whether the project's active set is the one this was resolved from.
    pub(super) fn is_current(&self, room: &RoomHandle) -> bool {
        self.epoch == room.epoch()
    }

    /// The pipelines `event` runs in `room`, in path order.
    pub(super) fn for_event<'a>(&'a self, room: &'a str, event: &'a str) -> impl Iterator<Item = &'a CompiledPipeline> + 'a {
        self.pipelines
            .iter()
            .map(|compiled| &**compiled)
            .filter(move |compiled| compiled.ws_triggers.iter().any(|t| admission::trigger_matches(&t.room, &t.event, room, event)))
    }
}

/// Run every pipeline whose `trigger.room` matches one event, one after
/// another. The identity is the one proved at join; nothing is re-checked.
pub(super) async fn run_event(ctx: &RoomCtx, room: &RoomHandle, resolved: &Resolved, event: RoomEvent) {
    // `$trigger` is the envelope `trigger.room` answers under `room`.
    let input = json!({
        "room_id": ctx.room_id,
        "session_id": event.session,
        "event": event.event,
        "payload": event.payload,
        "auth": event.auth,
    });
    for compiled in resolved.for_event(&ctx.room_id, &event.event) {
        run_pipeline(ctx, room, resolved, compiled, input.clone(), &event.session).await;
    }
}

/// One run of one pipeline for the room, counted into the room's counters.
pub(super) async fn run_pipeline(
    ctx: &RoomCtx,
    room: &RoomHandle,
    resolved: &Resolved,
    compiled: &CompiledPipeline,
    mut input: Value,
    run_of: &str,
) {
    // `--state`: the paths the matching triggers name, read now, after
    // every run before this one.
    let event = input.get("event").and_then(Value::as_str).unwrap_or_default().to_string();
    let paths: Vec<String> = compiled
        .ws_triggers
        .iter()
        .filter(|t| admission::trigger_matches(&t.room, &t.event, &ctx.room_id, &event))
        .flat_map(|t| t.state.iter().cloned())
        .collect();
    if !paths.is_empty()
        && let Some(envelope) = input.as_object_mut()
    {
        envelope.insert("state".to_string(), room.read_state(&paths));
    }
    let pctx = PipelineContext {
        owner: ctx.owner.clone(),
        project: ctx.project.clone(),
        pipeline: compiled.graph.id.clone(),
        request_id: format!("{run_of}-{}", uuid::Uuid::new_v4().simple()),
        route: Default::default(),
        input: input.clone(),
        trigger: Some(input),
        placeholder: None,
    };
    room.count(Counter::PipelineRuns);
    let started = std::time::Instant::now();
    let at = crate::platform::model::now_ts();
    // The graph is the active runtime's, validated at activation.
    // What a room run does is what it wrote and sent; its value is read by
    // no one, so the run keeps none.
    let options = crate::pipeline::model::ExecuteOptions {
        prevalidated: true,
        lane: Some(compiled.lane.clone()),
        discard_value: true,
        ..Default::default()
    };
    if let Err(err) = resolved.engine.execute_with_options_async(&compiled.graph, &pctx, &options).await {
        room.count(Counter::PipelineErrors);
        if ctx.state.platform.ws_hub.failure_due(&ctx.owner, &ctx.project, &compiled.file_rel_path, started) {
            record_failure(ctx, compiled, &pctx.request_id, at, started.elapsed(), &err);
        }
    }
}

/// A room run writes no run record; a failed one is recorded — run record,
/// error group, hit counter — at most once a minute per pipeline.
fn record_failure(
    ctx: &RoomCtx,
    compiled: &CompiledPipeline,
    run_id: &str,
    at: i64,
    took: std::time::Duration,
    err: &crate::pipeline::PipelineError,
) {
    let platform = &ctx.state.platform;
    let (owner, project, file) = (ctx.owner.as_str(), ctx.project.as_str(), compiled.file_rel_path.as_str());
    eprintln!("warning: room {} pipeline {file} failed: {} {}", ctx.room_id, err.code, err.message);
    platform.pipeline_hits.record_failure(owner, project, file, "room", &err.code, &err.message);
    let config = match platform.zebflow_cfg.read_or_default(owner, project) {
        Ok(config) => config,
        Err(e) => return eprintln!("warning: room failure not recorded for {owner}/{project}: {}", e.message),
    };
    let retention = crate::platform::model::resolve_invocation_retention(&config, Some(&compiled.graph));
    let entry = crate::platform::model::PipelineInvocationEntry {
        run_id: run_id.to_string(),
        at,
        duration_ms: took.as_millis() as u64,
        status: "error".to_string(),
        trigger: "room".to_string(),
        error: Some(err.message.clone()),
        trace: err.node_trace.clone(),
    };
    let bounds = config.configs.pipelines.logging.error_group_bounds();
    if let Err(e) = platform.data.record_pipeline_error(owner, project, file, &entry, bounds) {
        eprintln!("warning: error group not recorded for {owner}/{project}: {}", e.message);
    }
    let _ = platform.data.log_pipeline_invocation(owner, project, file, entry, retention.max_invocations, retention.max_age_secs);
}
