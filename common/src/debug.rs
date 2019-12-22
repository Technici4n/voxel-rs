use crossbeam_channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use std::{collections::BTreeMap, sync::Arc, sync::RwLock};
lazy_static! {
    static ref DEBUG_INFO: Arc<RwLock<Option<Sender<DebugInfoUnit>>>> = Arc::new(RwLock::new(None));
}

#[derive(Debug, Clone)]
struct DebugInfoUnit {
    pub section: String,
    pub id: String,
    pub part: DebugInfoPart,
}

#[derive(Debug, Clone)]
pub enum DebugInfoPart {
    Message(String),
    WorkerPerf(WorkerPerf),
    PerfBreakdown(String, Vec<(String, f64)>)
}

/// Helper struct allowing multiple threads to easily show debug info.
/// There can only be one active `DebugInfo` at any time.
pub struct DebugInfo {
    receiver: Receiver<DebugInfoUnit>,
    sections: BTreeMap<String, (bool, u32, BTreeMap<String, DebugInfoPart>)>,
    next_id: u32,
}

impl DebugInfo {
    /// Create a new `DebugInfo` struct and make it the current one.
    pub fn new_current() -> Self {
        let (sender, receiver) = unbounded();
        *DEBUG_INFO.write().unwrap() = Some(sender);
        Self {
            receiver,
            sections: BTreeMap::new(),
            next_id: 0,
        }
    }

    /// Get the debug info
    pub fn get_debug_info(&mut self) -> &mut BTreeMap<String, (bool, u32, BTreeMap<String, DebugInfoPart>)> {
        let Self { ref mut next_id, .. } = self;
        while let Ok(diu) = self.receiver.try_recv() {
            let (_, _, inner_map) = self.sections
                .entry(diu.section)
                .or_insert_with(|| {
                    *next_id += 1;
                    (false, *next_id - 1, BTreeMap::new())
                });
            inner_map.insert(diu.id, diu.part);
        }
        &mut self.sections
    }
}

/// Send a debug info message to the current `DebugInfo` if there is one
pub fn send_debug_info(section: impl ToString, id: impl ToString, message: impl ToString) {
    DEBUG_INFO.read().unwrap().as_ref().map(|sender| {
        sender
            .send(DebugInfoUnit {
                section: section.to_string(),
                id: id.to_string(),
                part: DebugInfoPart::Message(message.to_string()),
            })
            .unwrap()
    });
}

#[derive(Debug, Clone)]
pub struct WorkerPerf {
    pub name: String,
    pub micros_per_iter: f32,
    pub iter_per_sec: f32,
    pub efficiency: f32,
    pub pending: usize,
}

/// Send a debug info worker perf
pub fn send_worker_perf(section: impl ToString, id: impl ToString, name: impl ToString, micros_per_iter: f32, iter_per_sec: f32, pending: usize) {
    DEBUG_INFO.read().unwrap().as_ref().map(|sender| {
        sender
            .send(DebugInfoUnit {
                section: section.to_string(),
                id: id.to_string(),
                part: DebugInfoPart::WorkerPerf(WorkerPerf {
                    name: name.to_string(),
                    micros_per_iter,
                    iter_per_sec,
                    efficiency: micros_per_iter / 1_000_000.0 * iter_per_sec,
                    pending,
                }),
            })
            .unwrap()
    });
}

/// Send a debug info performance breakdown
pub fn send_perf_breakdown(section: impl ToString, id: impl ToString, name: impl ToString, breakdown: Vec<(String, f64)>) {
    DEBUG_INFO.read().unwrap().as_ref().map(|sender| {
        sender
            .send(DebugInfoUnit {
                section: section.to_string(),
                id: id.to_string(),
                part: DebugInfoPart::PerfBreakdown(name.to_string(), breakdown),
            })
            .unwrap()
    });
}