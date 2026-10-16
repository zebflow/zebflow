//! The fast lane (`docs/contracts/rooms.md` § The fast lane).
//!
//! Room work — every room socket's loop, every room's queue and ticks, the
//! pipeline runs a room starts and the state it sends — runs on a runtime of
//! its own, apart from the threads serving HTTP and other pipelines, so a
//! slow webhook never makes a game stutter. Its worker threads are named
//! `zebflow-rooms`; there are as many as the machine has cores, minus one
//! for everything else, and at least one. It starts with the first room work
//! and lives as long as the process.

use std::sync::OnceLock;

static LANE: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// The lane's worker threads: the machine's cores minus one, at least one.
pub fn lane_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |cores| cores.get().saturating_sub(1)).max(1)
}

/// The rooms' runtime; spawn room work here.
pub fn lane() -> &'static tokio::runtime::Handle {
    LANE.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(lane_threads())
            .thread_name("zebflow-rooms")
            .enable_all()
            .build()
            .expect("the rooms runtime starts")
    })
    .handle()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lane_leaves_one_core_and_has_at_least_one_thread() {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        assert_eq!(lane_threads(), cores.saturating_sub(1).max(1));
    }

    #[test]
    fn room_work_runs_on_the_lane_threads() {
        let name = lane().block_on(async { lane().spawn(async { std::thread::current().name().map(str::to_string) }).await.unwrap() });
        assert_eq!(name.as_deref(), Some("zebflow-rooms"));
    }
}
