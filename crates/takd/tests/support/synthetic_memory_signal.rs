use std::path::{Path, PathBuf};

const GIB: u64 = 1024 * 1024 * 1024;
const TOTAL_BYTES: u64 = 16 * GIB;

pub struct SyntheticMemorySignal {
    path: PathBuf,
}

impl SyntheticMemorySignal {
    pub fn healthy(root: &Path) -> Self {
        let signal = Self {
            path: root.join("memory-signal.json"),
        };
        signal.set_available_bytes(12 * GIB);
        signal
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn apply_pressure(&self) {
        self.set_available_bytes(GIB / 2);
    }

    pub fn apply_soft_pressure(&self) {
        self.set_available_bytes(2 * GIB);
    }

    pub fn recover(&self) {
        self.set_available_bytes(12 * GIB);
    }

    fn set_available_bytes(&self, available_bytes: u64) {
        let payload =
            format!(r#"{{"available_bytes":{available_bytes},"total_bytes":{TOTAL_BYTES}}}"#);
        std::fs::write(&self.path, payload).expect("write synthetic memory signal");
    }
}
