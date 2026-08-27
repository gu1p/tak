use std::path::PathBuf;
use std::sync::Mutex;

use sysinfo::{MemoryRefreshKind, RefreshKind, System};

use crate::daemon::remote::runtime::RemoteRuntimeConfig;

pub(super) trait MemorySignal: Send + Sync {
    fn read(&self) -> (u64, u64);
}

pub(super) fn configured_memory_signal(
    runtime_config: &RemoteRuntimeConfig,
) -> Box<dyn MemorySignal> {
    match runtime_config.test_memory_signal_path() {
        Some(path) => Box::new(FileMemorySignal {
            path: path.to_path_buf(),
        }),
        None => Box::new(SysinfoMemorySignal::new()),
    }
}

struct FileMemorySignal {
    path: PathBuf,
}

impl MemorySignal for FileMemorySignal {
    fn read(&self) -> (u64, u64) {
        let Ok(payload) = std::fs::read_to_string(&self.path) else {
            return (0, 0);
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) else {
            return (0, 0);
        };
        (
            value["available_bytes"].as_u64().unwrap_or(0),
            value["total_bytes"].as_u64().unwrap_or(0),
        )
    }
}

struct SysinfoMemorySignal {
    system: Mutex<System>,
}

impl SysinfoMemorySignal {
    fn new() -> Self {
        Self {
            system: Mutex::new(System::new_with_specifics(
                RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
            )),
        }
    }
}

impl MemorySignal for SysinfoMemorySignal {
    fn read(&self) -> (u64, u64) {
        let Ok(mut system) = self.system.lock() else {
            return (0, 0);
        };
        system.refresh_memory();
        (system.available_memory(), system.total_memory())
    }
}
