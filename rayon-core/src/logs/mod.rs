//! Most code related to tasks logs is here.

mod common_types;
pub use common_types::{RawEvent, RawLogs, SubGraphId, TaskId, TimeStamp};
use std::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
lazy_static! {
    static ref START_TIME: std::time::Instant = std::time::Instant::now();
}

/// Return number of nano seconds since start.
pub(crate) fn now() -> TimeStamp {
    START_TIME.elapsed().as_nanos() as TimeStamp
}

/// Add given event to logs of current thread.
pub(super) fn log(event: RawEvent<&'static str>) {
    recorder::THREAD_LOGS.with(|l| l.push(event))
}

/// Logs several events at once (with decreased cost).
macro_rules! logs {
    ($($x:expr ), +) => {
        $crate::logs::recorder::THREAD_LOGS.with(|l| {
            $(
                l.push($x);
              )*
        })
    }
}

/// We use an atomic usize to generate unique ids for tasks.
/// We start at 1 since initial task (0) is created manually.
pub(crate) static NEXT_TASK_ID: AtomicUsize = AtomicUsize::new(1);

/// get an id for a new task and increment global tasks counter.
pub(super) fn next_task_id() -> TaskId {
    NEXT_TASK_ID.fetch_add(1, Ordering::SeqCst)
}

mod list;
pub(crate) mod recorder; // TODO: change pub
pub use recorder::Logger;
// pub(crate) mod scope;
mod storage;
pub(crate) use storage::Storage; // TODO: how to solve that ?
mod subgraphs;
pub use subgraphs::{custom_subgraph, subgraph};
