//! The queue statistics are computed from. At most one computation runs at a
//! time in the process, on a thread of its own, so statistics never compete
//! with each other and never run on a request.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};

use datafusion::prelude::{SessionConfig, SessionContext};

use super::saved::{self, Fingerprint, SavedStats};

/// One layer's computation: the GeoParquet file and where its statistics are
/// saved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub source: PathBuf,
    pub stats_file: PathBuf,
    /// Compute even when a saved result already describes this version of the
    /// file. Publishing sets it, so republishing retries a failed computation.
    pub force: bool,
}

pub struct StatsQueue {
    state: Mutex<State>,
    ready: Condvar,
}

struct State {
    waiting: VecDeque<Job>,
    running: Option<Job>,
}

impl StatsQueue {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State {
                waiting: VecDeque::new(),
                running: None,
            }),
            ready: Condvar::new(),
        }
    }

    /// Queues `job`. A job already waiting for the same statistics file is
    /// replaced by the newer one (keeping `force` if either had it); a job
    /// for the computation running now is not queued twice unless forced.
    pub fn enqueue(&self, job: Job) {
        let mut state = self.lock();
        if let Some(waiting) = state
            .waiting
            .iter_mut()
            .find(|w| w.stats_file == job.stats_file)
        {
            let force = waiting.force || job.force;
            *waiting = Job { force, ..job };
            return;
        }
        let same_as_running = state
            .running
            .as_ref()
            .is_some_and(|r| r.stats_file == job.stats_file && r.source == job.source);
        if same_as_running && !job.force {
            return;
        }
        state.waiting.push_back(job);
        drop(state);
        self.ready.notify_one();
    }

    /// The jobs waiting, oldest first.
    #[cfg(test)]
    pub fn waiting(&self) -> Vec<Job> {
        self.lock().waiting.iter().cloned().collect()
    }

    /// Takes the oldest waiting job and marks it running.
    #[cfg(test)]
    pub(super) fn take(&self) -> Option<Job> {
        let mut state = self.lock();
        let job = state.waiting.pop_front()?;
        state.running = Some(job.clone());
        Some(job)
    }

    /// Waits for a job, then takes it as [`Self::take`] does.
    fn take_blocking(&self) -> Job {
        let mut state = self.lock();
        loop {
            if let Some(job) = state.waiting.pop_front() {
                state.running = Some(job.clone());
                return job;
            }
            state = self.ready.wait(state).unwrap_or_else(|e| e.into_inner());
        }
    }

    pub(super) fn finish(&self) {
        self.lock().running = None;
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs every waiting job on this thread, in order, with sessions from
    /// `make_session`. Answers how many ran. For tests, which drive the
    /// queue by hand instead of through the process's worker.
    #[cfg(test)]
    pub fn run_waiting(&self, make_session: &dyn Fn(SessionConfig) -> SessionContext) -> usize {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a test runtime");
        let mut ran = 0;
        while let Some(job) = self.take() {
            run(&runtime, &job, make_session);
            self.finish();
            ran += 1;
        }
        ran
    }
}

/// The process's queue. Its worker thread starts on first use.
pub fn global() -> &'static StatsQueue {
    static QUEUE: StatsQueue = StatsQueue::new();
    static WORKER: OnceLock<()> = OnceLock::new();
    WORKER.get_or_init(|| {
        let spawned = std::thread::Builder::new()
            .name("mapserver-stats".to_string())
            .spawn(|| work(&QUEUE));
        if let Err(err) = spawned {
            eprintln!("mapserver: the statistics worker did not start: {err}");
        }
    });
    &QUEUE
}

fn work(queue: &'static StatsQueue) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("mapserver: the statistics worker has no runtime: {err}");
            return;
        }
    };
    loop {
        let job = queue.take_blocking();
        run(
            &runtime,
            &job,
            &crate::mapserver::resolve::geoparquet::new_session,
        );
        queue.finish();
    }
}

/// Computes one job's statistics and saves them, or saves why they could not
/// be computed. Skipped when the file is gone, or when a saved result already
/// describes this version of it and the job is not forced.
fn run(
    runtime: &tokio::runtime::Runtime,
    job: &Job,
    make_session: &dyn Fn(SessionConfig) -> SessionContext,
) {
    let Some(source) = Fingerprint::of(&job.source) else {
        return;
    };
    if !job.force && saved::read(&job.stats_file).is_some_and(|s| s.source == source) {
        return;
    }
    let computed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ctx = make_session(SessionConfig::new());
        runtime.block_on(super::compute::compute(&job.source, &ctx))
    }))
    .unwrap_or_else(|_| Err("the statistics computation stopped unexpectedly".to_string()));
    if let Err(err) = saved::write(&job.stats_file, &SavedStats::new(source, computed)) {
        eprintln!("mapserver: statistics were not saved: {err}");
    }
}
