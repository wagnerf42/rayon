//! Types which are common between rayon and rayon-logs.

/// unique subgraph identifier
pub type SubGraphId = usize;
/// unique task identifier
pub type TaskId = usize;
/// at which time (in nanoseconds) does the event happen
pub type TimeStamp = u64;

/// All types of raw events we can log.
/// It is generic because recorded logs and reloaded logs
/// don't use the same strings for subgraphs.
#[derive(Debug, Clone)]
pub enum RawEvent<S> {
    /// A task starts.
    TaskStart(TaskId, TimeStamp),
    /// Active task ends.
    TaskEnd(TimeStamp),
    /// Direct link in the graph between two tasks (active one and given one).
    Child(TaskId),
    /// Start a subgraph.
    SubgraphStart(S),
    /// End a subgraph and register a work amount.
    SubgraphEnd(S, usize),
}

/// Raw unprocessed logs. Very fast to record but require some postprocessing to be displayed.
#[derive(Debug)]
pub struct RawLogs {
    /// A vector containing for each thread a vector of all recorded events.
    pub thread_events: Vec<Vec<RawEvent<SubGraphId>>>,
    /// All labels used for tagging subgraphs.
    pub labels: Vec<String>,
}
