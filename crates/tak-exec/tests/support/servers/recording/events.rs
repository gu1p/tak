use std::sync::{Arc, Mutex};

use tak_proto::SubmitTaskRequest;

#[derive(Clone, Default)]
pub struct RecordingEvents {
    entries: Arc<Mutex<Vec<String>>>,
    submit_payloads: Arc<Mutex<Vec<SubmitTaskRequest>>>,
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

    pub fn record_submit_payload(&self, payload: SubmitTaskRequest) {
        self.submit_payloads
            .lock()
            .expect("submit payload recorder lock")
            .push(payload);
    }

    pub fn submit_payloads(&self) -> Vec<SubmitTaskRequest> {
        self.submit_payloads
            .lock()
            .expect("submit payload recorder lock")
            .clone()
    }
}
