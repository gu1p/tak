use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct RecordingEvents {
    entries: Arc<Mutex<Vec<String>>>,
}

impl RecordingEvents {
    pub fn record(&self, entry: impl Into<String>) {
        self.entries
            .lock()
            .expect("event recorder lock")
            .push(entry.into());
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.entries.lock().expect("event recorder lock").clone()
    }
}
