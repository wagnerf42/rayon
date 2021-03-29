//! Most code related to tasks logs is here.
//! The only other modifications to rayon are :
//! - for associating thread logs to `Logger` struct (in registry.rs)
//! - calling logs functions for each operation implying tasks (join, install, spawn, scope,...)

mod common_types;
pub use common_types::{RawEvent, RawLogs, SubGraphId, TaskId, TimeStamp};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

// tasks ids data and function
// ---------------------------

/// We use an atomic usize to generate unique ids for tasks.
/// We start at 1 since initial task (0) is created manually.
static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

/// get an id for a new task and increment global tasks counter.
pub(super) fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

// timing data and function
// ------------------------

use lazy_static::lazy_static;
lazy_static! {
    static ref START_TIME: std::time::Instant = std::time::Instant::now();
}

/// Return number of nano seconds since start.
pub(super) fn now() -> TimeStamp {
    START_TIME.elapsed().as_nanos() as TimeStamp
}

// logging data and functions
// --------------------------

thread_local! {
    /// each thread has a storage space for logs
    pub(super) static THREAD_LOGS: Arc<Storage<RawEvent<&'static str>>> =  {
        Arc::new(Storage::new())
    };
}

/// Add given event to logs of current thread.
pub(super) fn log(event: RawEvent<&'static str>) {
    THREAD_LOGS.with(|l| l.push(event))
}

/// Logs several events at once (with decreased cost).
macro_rules! logs {
    ($($x:expr ), +) => {
        $crate::tasks_logs::THREAD_LOGS.with(|l| {
            $(
                l.push($x);
              )*
        })
    }
}

// define and re-export subgraphs functions
mod subgraphs;
pub use subgraphs::{custom_subgraph, subgraph};

// define and re-export `Storage` structure
mod list;
mod storage;
pub(super) use storage::Storage;

// define and re-export the main public structure : `Logger`
pub mod logger;
pub use logger::Logger;
