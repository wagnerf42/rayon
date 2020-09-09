//! Main struct for accesses to logs.
//! Structs, functions and global variables for recording logs.
use super::log;
use super::next_task_id;
use super::now;
use super::storage::Storage;
use super::{RawEvent, RawLogs, SubGraphId, TaskId};
use std::collections::HashMap;
use std::collections::LinkedList;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};

thread_local! {
    /// each thread has a storage space for logs
    //TODO: change pub crate
    pub(crate) static THREAD_LOGS: Arc<Storage<RawEvent<&'static str>>> =  {
        Arc::new(Storage::new())
    };
}

impl RawLogs {
    /// Extract recorded events and reset structs.
    /// It's better to do it when no events are being recorded.
    /// We are able to extract logs during recording but the obtained logs
    /// might be incomplete.
    pub(crate) fn new(logger: &Logger) -> Self {
        // stop main task
        log(RawEvent::TaskEnd(now()));
        // associate a unique integer id to each label
        let mut next_label_count = 0;
        let mut seen_labels = HashMap::new();
        let mut labels = Vec::new();
        let mut thread_events: Vec<Vec<RawEvent<SubGraphId>>> = Vec::new();
        // loop on all logged  rayon events per thread
        for thread_logs in logger.logs.lock().unwrap().iter() {
            let mut events = Vec::new();
            for rayon_event in thread_logs.iter() {
                // store eventual event label
                match rayon_event {
                    RawEvent::SubgraphStart(label) | RawEvent::SubgraphEnd(label, _) => {
                        seen_labels.entry(*label).or_insert_with(|| {
                            let label_count = next_label_count;
                            next_label_count += 1;
                            labels.push(label.to_string());
                            label_count
                        });
                    }
                    _ => (),
                }
                // convert to raw_event with stored label
                let raw_event = RawEvent::new(rayon_event, &seen_labels);
                events.push(raw_event);
            }
            thread_events.push(events);
        }

        // now we just need to turn the hash table into a vector, filling the gaps
        // if some threads registered no events yet
        RawLogs {
            thread_events,
            labels,
        }
    }
    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let mut file = File::create(path)?;
        // we start by saving all labels
        write_vec_strings_to(&self.labels, &mut file)?;
        // write the number of threads
        write_u64(self.thread_events.len() as u64, &mut file)?;
        // now, all events
        for events in &self.thread_events {
            write_u64(events.len() as u64, &mut file)?; // how many events for this thread
            events.iter().try_for_each(|e| e.write_to(&mut file))?;
        }
        Ok(())
    }
}

// little endian write
fn write_u64<W: std::io::Write>(integer: u64, destination: &mut W) -> std::io::Result<()> {
    let mut remaining = integer;
    for _ in 0..8 {
        let low_bits = (remaining & 255) as u8;
        remaining = remaining >> 8;
        destination.write(&[low_bits])?;
    }
    Ok(())
}

fn write_vec_strings_to<W: std::io::Write>(
    vector: &Vec<String>,
    destination: &mut W,
) -> std::io::Result<()> {
    // write the length
    write_u64(vector.len() as u64, destination)?;
    // write for each string its byte size and then all bytes
    for string in vector {
        let bytes = string.as_bytes();
        write_u64(string.len() as u64, destination)?;
        destination.write(bytes)?;
    }
    Ok(())
}

impl RawEvent<TaskId> {
    pub(crate) fn new(
        rayon_event: &RawEvent<&'static str>,
        strings: &HashMap<&str, usize>,
    ) -> RawEvent<TaskId> {
        match rayon_event {
            RawEvent::TaskStart(id, time) => RawEvent::TaskStart(*id, *time),
            RawEvent::TaskEnd(time) => RawEvent::TaskEnd(*time),
            RawEvent::Child(id) => RawEvent::Child(*id),
            RawEvent::SubgraphStart(label) => RawEvent::SubgraphStart(strings[label]),
            RawEvent::SubgraphEnd(label, size) => RawEvent::SubgraphEnd(strings[label], *size),
        }
    }
    pub(crate) fn write_to<W: std::io::Write>(&self, destination: &mut W) -> std::io::Result<()> {
        match self {
            RawEvent::TaskStart(id, time) => {
                destination.write(&[2u8])?;
                write_u64(*id as u64, destination)?;
                write_u64(*time, destination)?;
            }
            RawEvent::TaskEnd(time) => {
                destination.write(&[3u8])?;
                write_u64(*time, destination)?;
            }
            RawEvent::Child(id) => {
                destination.write(&[4u8])?;
                write_u64(*id as u64, destination)?;
            }
            RawEvent::SubgraphStart(label) => {
                destination.write(&[5u8])?;
                write_u64(*label as u64, destination)?;
            }
            RawEvent::SubgraphEnd(label, size) => {
                destination.write(&[6u8])?;
                write_u64(*label as u64, destination)?;
                write_u64(*size as u64, destination)?;
            }
        }
        Ok(())
    }
}

/// This is the main structure for logging in rayon.
#[derive(Debug)]
pub struct Logger {
    /// All logs are registered here.
    logs: Arc<Mutex<LinkedList<Arc<Storage<RawEvent<&'static str>>>>>>,
}

impl Logger {
    /// Create a new global logger.
    /// The thread calling this method will get logged in addition
    /// to all threads obtained from `pool_builder` method.
    pub fn new() -> Self {
        let logs = Arc::new(Mutex::new(LinkedList::new()));
        {
            logs.lock().unwrap().push_front(THREAD_LOGS.with(|l| {
                l.push(RawEvent::TaskStart(0, now()));
                l.clone()
            }));
        }
        Logger { logs }
    }
    /// Create a `ThreadPoolBuilder` whose pool will be logged.
    pub fn pool_builder(&self) -> crate::ThreadPoolBuilder {
        let mut builder: crate::ThreadPoolBuilder = Default::default();
        builder.tasks_logger = Some(self.logs.clone());
        builder
    }
    /// Extract recorded logs (removing them from records).
    pub fn extract_logs(&self) -> RawLogs {
        RawLogs::new(self)
    }
    /// Erase all logs and restart logging.
    pub fn reset(&self) {
        self.logs.lock().unwrap().iter().for_each(|log| log.reset());
        log(RawEvent::TaskStart(next_task_id(), now()));
    }

    /// Save log file of currently recorded raw logs.
    /// This will reset logs.
    pub fn save_raw_logs<P: AsRef<Path>>(&mut self, path: P) -> Result<(), io::Error> {
        let logs = RawLogs::new(self);
        logs.save(path)?;
        self.reset();
        Ok(())
    }
}
