pub(super) struct LineFramer {
    pending: Vec<u8>,
    limit: usize,
}

impl LineFramer {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            pending: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(bytes);
        let mut framed = self.take_complete_lines();
        while self.pending.len() > self.limit {
            framed.push(self.pending.drain(..self.limit).collect());
        }
        framed
    }

    pub(super) fn finish(&mut self) -> Vec<Vec<u8>> {
        if self.pending.is_empty() {
            Vec::new()
        } else {
            vec![std::mem::take(&mut self.pending)]
        }
    }

    fn take_complete_lines(&mut self) -> Vec<Vec<u8>> {
        let mut framed = Vec::new();
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            framed.push(self.pending.drain(..=end).collect());
        }
        framed
    }
}
