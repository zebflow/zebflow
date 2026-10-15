//! One session's events, run strictly in order, each by every pipeline
//! whose `trigger.room` matches it.

use std::sync::Arc;

use super::*;
use crate::infra::transport::ws::{Counter, RoomHandle};

/// Most events one session may have waiting for its pipelines.
pub(super) const MAX_PENDING_SESSION_EVENTS: usize = 256;

pub(super) type SessionEvent = (String, Value);

pub(super) fn session_event_channel() -> (
    tokio::sync::mpsc::Sender<SessionEvent>,
    tokio::sync::mpsc::Receiver<SessionEvent>,
) {
    tokio::sync::mpsc::channel(MAX_PENDING_SESSION_EVENTS)
}

/// Queues one event without waiting; `false` when the queue is full or closed.
pub(super) fn enqueue_event(tx: &tokio::sync::mpsc::Sender<SessionEvent>, event: String, payload: Value) -> bool {
    tx.try_send((event, payload)).is_ok()
}

/// Run one session's events strictly in order.
pub(super) async fn run_session_events(
    ctx: SessionCtx,
    room: Arc<RoomHandle>,
    auth: Value,
    mut events: tokio::sync::mpsc::Receiver<SessionEvent>,
) {
    while let Some((event, payload)) = events.recv().await {
        run_event(&ctx, &room, &auth, &event, payload).await;
    }
}

/// Run every pipeline whose `trigger.room` matches `event`, one after
/// another. The identity is the one proved at join; nothing is re-checked.
async fn run_event(ctx: &SessionCtx, room: &RoomHandle, auth: &Value, event: &str, payload: Value) {
    let pipelines = ctx.state.platform.pipeline_runtime.list_project(&ctx.owner, &ctx.project);
    for compiled in pipelines {
        if !compiled
            .ws_triggers
            .iter()
            .any(|t| admission::trigger_matches(&t.room, &t.event, &ctx.room_id, event))
        {
            continue;
        }
        // `$trigger` is the envelope `trigger.room` answers under `room`.
        let input = json!({
            "room_id": ctx.room_id,
            "session_id": ctx.session_id,
            "event": event,
            "payload": payload,
            "auth": auth,
        });
        let pctx = PipelineContext {
            owner: ctx.owner.clone(),
            project: ctx.project.clone(),
            pipeline: compiled.graph.id.clone(),
            request_id: format!("{}-{}", ctx.session_id, uuid::Uuid::new_v4().simple()),
            route: Default::default(),
            input: input.clone(),
            trigger: Some(input),
            placeholder: None,
        };
        let state = &ctx.state;
        let engine = crate::pipeline::BasicPipelineEngine::new(
            std::sync::Arc::new(state.platform.project_sandbox(&ctx.owner, &ctx.project)),
            state.frontend.rwe.clone(),
            Some(state.platform.credentials.clone()),
        )
        .with_platform(state.platform.clone())
        .with_ws_hub(state.platform.ws_hub.clone())
        .with_ws_client_manager(state.ws_client_manager.clone())
        .with_state_bus(state.platform.state_bus.clone())
        .with_data_root(state.platform.config.data_root.clone());
        room.count(Counter::PipelineRuns);
        if let Err(err) = engine.execute_async(&compiled.graph, &pctx).await {
            room.count(Counter::PipelineErrors);
            eprintln!(
                "warning: ws pipeline {} ({}) failed on {}: {} {}",
                compiled.graph.id, ctx.room_id, event, err.code, err.message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_session_event_queue_is_bounded_and_refuses_past_its_capacity() {
        let (tx, _rx) = session_event_channel();
        for i in 0..MAX_PENDING_SESSION_EVENTS {
            assert!(enqueue_event(&tx, "move".into(), json!({ "i": i })), "event {i} fits");
        }
        assert!(
            !enqueue_event(&tx, "move".into(), json!({})),
            "one past the cap is refused, not queued without limit"
        );
    }
}
