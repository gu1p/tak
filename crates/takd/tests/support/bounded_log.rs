use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
#[cfg(not(target_os = "macos"))]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub const LARGE_LOG_BYTES: u64 = 2 * 1024 * 1024 * 1024;
#[cfg(not(target_os = "macos"))]
pub const LOG_DATA_LIMIT_BYTES: u64 = 512 * 1024 * 1024;

pub fn write_sparse_log(path: &Path, tail: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .expect("create sparse service log");
    file.set_len(LARGE_LOG_BYTES).expect("size sparse log");
    file.seek(SeekFrom::End(-(tail.len() as i64)))
        .expect("seek to sparse log tail");
    file.write_all(tail.as_bytes()).expect("write log tail");
}

pub fn command_with_data_limit(program: &Path) -> Command {
    let command = Command::new(program);
    // macOS rejects finite RLIMIT_DATA values with EINVAL. The sparse-file
    // contract still exercises the suffix path there; other Unix hosts also
    // enforce that the child cannot allocate the complete fixture.
    #[cfg(not(target_os = "macos"))]
    // SAFETY: the child-only hook calls setrlimit, which is async-signal-safe,
    // and captures only a constant scalar value.
    let command = unsafe {
        let mut command = command;
        command.pre_exec(|| {
            let bytes = LOG_DATA_LIMIT_BYTES as libc::rlim_t;
            let limit = libc::rlimit {
                rlim_cur: bytes,
                rlim_max: bytes,
            };
            if libc::setrlimit(libc::RLIMIT_DATA, &limit) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
        command
    };
    command
}
