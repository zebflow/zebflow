//! How far each JavaScript runtime's heap may grow.
//!
//! Left to itself V8 sizes a heap from the machine's physical memory, which a
//! container's memory limit does not change: on a 1 GB device inside a larger
//! host every runtime was allowed about 1.5 GB, so a script that kept
//! allocating was stopped by the kernel killing the whole process, every
//! project on it included, before V8 ever said no. Here the instance sets one
//! limit for every runtime it makes, the script sandbox's and the page
//! renderer's alike, and a script that reaches it fails alone.
//!
//! A runtime is made with the limit current when it is made. Changing the
//! limit starts a new generation; a runtime of an older one is replaced before
//! its next run, so no run is cut short by a change.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use deno_core::{JsRuntime, v8};

/// The heap limit in bytes; 0 is V8's own choice.
static LIMIT: AtomicUsize = AtomicUsize::new(0);
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// How far a script past its limit may still grow while it is being
/// stopped, as a multiple of the limit: termination takes effect at the next
/// check, and V8 aborts the process if it cannot allocate before then.
const STOPPING_ROOM: usize = 4;

/// Sets the heap each runtime made from now on may grow to, in bytes; 0
/// leaves it to V8. Runtimes already running are replaced before their next
/// run.
pub fn set_runtime_heap_limit(bytes: usize) {
    if LIMIT.swap(bytes, Ordering::AcqRel) != bytes {
        GENERATION.fetch_add(1, Ordering::AcqRel);
    }
}

/// The heap limit runtimes are made with now, in bytes; 0 is V8's own.
pub fn runtime_heap_limit() -> usize {
    LIMIT.load(Ordering::Acquire)
}

/// The generation runtimes made now belong to.
pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

/// The heap a runtime is being made with.
pub(crate) struct Heap {
    /// For `RuntimeOptions::create_params`.
    pub params: Option<v8::CreateParams>,
    /// The generation it belongs to.
    pub generation: u64,
    /// Its limit in bytes; 0 is V8's own.
    pub limit: usize,
}

/// The heap for a runtime made now.
pub(crate) fn current() -> Heap {
    let generation = generation();
    let limit = runtime_heap_limit();
    Heap { params: params(limit), generation, limit }
}

fn params(limit: usize) -> Option<v8::CreateParams> {
    (limit > 0).then(|| v8::CreateParams::default().heap_limits(0, limit))
}

/// A limit as an error names it: `36MiB`, or V8's own.
pub(crate) fn describe(limit: usize) -> String {
    match limit {
        0 => "V8's default".to_string(),
        bytes => format!("{}MiB", bytes >> 20),
    }
}

/// Stops a script that reaches `limit` instead of letting V8 abort the
/// process. The returned flag is set once it has; the runtime is then left
/// with a terminated script and a heap near full, and is retired.
pub(crate) fn stop_at_heap_limit(js_rt: &mut JsRuntime, limit: usize) -> Rc<Cell<bool>> {
    let reached = Rc::new(Cell::new(false));
    let flag = reached.clone();
    let handle = js_rt.v8_isolate().thread_safe_handle();
    let ceiling = limit.saturating_mul(STOPPING_ROOM);
    js_rt.add_near_heap_limit_callback(move |current, _initial| {
        flag.set(true);
        handle.terminate_execution();
        let next = current.saturating_mul(2);
        if ceiling == 0 { next } else { next.min(ceiling).max(current) }
    });
    reached
}

#[cfg(test)]
mod tests {
    use super::*;
    use deno_core::{FastString, RuntimeOptions};

    #[test]
    fn a_script_that_reaches_its_heap_limit_fails_alone() {
        let limit = 32 << 20;
        let mut js_rt = JsRuntime::new(RuntimeOptions { create_params: params(limit), ..Default::default() });
        let reached = stop_at_heap_limit(&mut js_rt, limit);
        let runaway = "const kept = []; while (true) kept.push(new Array(100000).fill(1));";
        let result = js_rt.execute_script("<runaway>", FastString::from_static(runaway));
        assert!(result.is_err(), "the script was stopped");
        assert!(reached.get(), "by its heap limit");
        // The process is still here, and a runtime made after it works.
        let mut next = JsRuntime::new(RuntimeOptions { create_params: params(limit), ..Default::default() });
        assert!(next.execute_script("<after>", FastString::from_static("1 + 1")).is_ok());
    }

    #[test]
    fn the_same_limit_again_is_not_a_new_generation() {
        let before = generation();
        set_runtime_heap_limit(runtime_heap_limit());
        assert_eq!(generation(), before);
        assert_eq!(describe(36 << 20), "36MiB");
        assert_eq!(describe(0), "V8's default");
    }
}
