use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, anyhow};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::writer::MakeWriter;

const SERVICE_LOG_FILE: &str = "service.log";

pub fn service_log_path(state_root: &Path) -> PathBuf {
    state_root.join(SERVICE_LOG_FILE)
}

pub fn init_service_logging(state_root: &Path) -> Result<()> {
    fs::create_dir_all(state_root)
        .with_context(|| format!("create takd state root {}", state_root.display()))?;
    let log_path = service_log_path(state_root);
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open takd service log {}", log_path.display()))?;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(SharedFileWriter::new(file))
        .try_init()
        .map_err(|err| anyhow!("initialize takd service logger: {err}"))?;
    Ok(())
}

pub fn read_service_log_tail(state_root: &Path, lines: usize) -> Result<String> {
    let log_path = service_log_path(state_root);
    let contents = fs::read_to_string(&log_path)
        .with_context(|| format!("service log not found at {}", log_path.display()))?;
    Ok(tail_lines(&contents, lines))
}

fn tail_lines(contents: &str, lines: usize) -> String {
    if lines == 0 || contents.is_empty() {
        return String::new();
    }

    let all_lines = contents.lines().collect::<Vec<_>>();
    let start = all_lines.len().saturating_sub(lines);
    let mut tail = all_lines[start..].join("\n");
    if !tail.is_empty() && contents.ends_with('\n') {
        tail.push('\n');
    }
    tail
}

#[derive(Clone)]
struct SharedFileWriter {
    file: Arc<Mutex<File>>,
}

impl SharedFileWriter {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

struct LockedFileWriter<'a> {
    guard: MutexGuard<'a, File>,
}

impl Write for LockedFileWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.guard.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.guard.flush()
    }
}

impl<'a> MakeWriter<'a> for SharedFileWriter {
    type Writer = LockedFileWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        LockedFileWriter {
            guard: self.file.lock().expect("takd service log mutex poisoned"),
        }
    }
}
