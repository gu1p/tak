#[derive(Clone)]
struct RemoteWorkerEventWriter {
    state: Arc<std::sync::Mutex<RemoteWorkerEventWriterState>>,
}

struct RemoteWorkerEventWriterState {
    store: SubmitAttemptStore,
    idempotency_key: String,
    next_seq: u64,
}

impl RemoteWorkerEventWriter {
    fn new(
        store: SubmitAttemptStore,
        idempotency_key: String,
        next_seq: u64,
    ) -> Self {
        Self {
            state: Arc::new(std::sync::Mutex::new(RemoteWorkerEventWriterState {
                store,
                idempotency_key,
                next_seq,
            })),
        }
    }

    fn append(&self, payload: serde_json::Value) -> Result<u64> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow!("remote worker event writer lock poisoned"))?;
        let seq = state.next_seq;
        state
            .store
            .append_event(&state.idempotency_key, seq, &payload.to_string())?;
        state.next_seq = state.next_seq.saturating_add(1);
        Ok(seq)
    }
}
