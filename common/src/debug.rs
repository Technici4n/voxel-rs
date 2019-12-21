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
    pub message: String,
}

/// Helper struct allowing multiple threads to easily show debug info.
/// There can only be one active `DebugInfo` at any time.
pub struct DebugInfo {
    receiver: Receiver<DebugInfoUnit>,
    sections: BTreeMap<String, (bool, u32, BTreeMap<String, String>)>,
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
    pub fn get_debug_info(&mut self) -> &mut BTreeMap<String, (bool, u32, BTreeMap<String, String>)> {
        let Self { ref mut next_id, .. } = self;
        while let Ok(diu) = self.receiver.try_recv() {
            let (_, _, inner_map) = self.sections
                .entry(diu.section)
                .or_insert_with(|| {
                    *next_id += 1;
                    (false, *next_id - 1, BTreeMap::new())
                });
            let stored_message = inner_map
                .entry(diu.id)
                .or_insert_with(String::new);
            *stored_message = diu.message;
        }
        &mut self.sections
    }
}

/// Send debug info to the current `DebugInfo` if there is one
pub fn send_debug_info(section: impl ToString, id: impl ToString, message: impl ToString) {
    DEBUG_INFO.read().unwrap().as_ref().map(|sender| {
        sender
            .send(DebugInfoUnit {
                section: section.to_string(),
                id: id.to_string(),
                message: message.to_string(),
            })
            .unwrap()
    });
}
