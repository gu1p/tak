use std::io::{self, IsTerminal};
use std::sync::atomic::{AtomicBool, Ordering};

use tak_exec::OutputStream;

pub(super) struct OutputVisibility {
    dashboard_active: AtomicBool,
    stdout_terminal: bool,
}

impl OutputVisibility {
    pub(super) fn current() -> Self {
        Self::for_attachment(false, io::stdout().is_terminal())
    }

    pub(super) fn for_attachment(dashboard_active: bool, stdout_terminal: bool) -> Self {
        Self {
            dashboard_active: AtomicBool::new(dashboard_active),
            stdout_terminal,
        }
    }

    pub(super) fn set_dashboard_active(&self, active: bool) {
        self.dashboard_active.store(active, Ordering::SeqCst);
    }

    pub(super) fn writes(&self, stream: OutputStream) -> bool {
        let dashboard_active = self.dashboard_active.load(Ordering::SeqCst);
        match stream {
            OutputStream::Stdout => !dashboard_active || !self.stdout_terminal,
            OutputStream::Stderr => !dashboard_active,
        }
    }
}
