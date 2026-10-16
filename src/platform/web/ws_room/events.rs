//! What a room runs: the pipelines resolved for it, and one run of one.
//!
//! The pipelines whose `trigger.room` matches a room are resolved when its
//! loop starts and again when the project's active set changes
//! ([`RoomHandle::epoch`]) — never per message (`rooms.md` § The fast lane).

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
    pipelines: Vec<CompiledPipeline>,
    engine: crate::pipeline::BasicPipelineEngine,
}

impl Resolved {
    /// Resolve the room's pipelines from the project's active set now.
    pub(super) fn of(ctx: &RoomCtx, room: &RoomHandle) -> Self {
        // Read the epoch first: a change after it raises it again, and the
        // loop resolves once more.
        let epoch = room.epoch();
        let mut pipelines: Vec<CompiledPipeline> = ctx
            .state
            .platform
            .pipeline_runtime
            .list_project(&ctx.owner, &ctx.project)
            .into_iter()
            .filter(|compiled| compiled.ws_triggers.iter().any(|t| admission::declares(&t.room, &ctx.room_id)))
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
    input: Value,
    run_of: &str,
) {
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
    if let Err(err) = resolved.engine.execute_async(&compiled.graph, &pctx).await {
        room.count(Counter::PipelineErrors);
        eprintln!(
            "warning: ws pipeline {} ({}) failed: {} {}",
            compiled.graph.id, ctx.room_id, err.code, err.message
        );
    }
}
