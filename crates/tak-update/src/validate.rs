//! Pre-swap validation of a candidate binary by its `--version` output.
//!
//! Before any live binary is replaced, the freshly-downloaded candidate is
//! executed with `--version` and its output is required to match the release tag
//! exactly. This is the cheapest, highest-value guard: a corrupt or wrong-arch
//! download is caught before a single byte of the running binary is touched.

use std::io;
use std::path::Path;
use std::process::Command;

/// The exact `--version` line a binary must print for a given tag.
///
/// The tag carries a leading `v` (`v0.1.7`) while `--version` output does not
/// (`tak 0.1.7`), so the `v` is stripped.
///
/// ```rust
/// use tak_update::validate::expected_version_line;
/// assert_eq!(expected_version_line("takd", "v0.1.7"), "takd 0.1.7");
/// assert_eq!(expected_version_line("tak", "0.1.7"), "tak 0.1.7");
/// ```
pub fn expected_version_line(name: &str, tag: &str) -> String {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    format!("{name} {version}")
}

/// Whether `stdout` from `<name> --version` matches `tag` (trimming surrounding whitespace).
///
/// ```rust
/// use tak_update::validate::version_output_matches;
/// assert!(version_output_matches("takd 0.1.7\n", "takd", "v0.1.7"));
/// assert!(!version_output_matches("takd 0.1.6\n", "takd", "v0.1.7"));
/// ```
pub fn version_output_matches(stdout: &str, name: &str, tag: &str) -> bool {
    stdout.trim() == expected_version_line(name, tag)
}

/// Execute `path --version` and capture stdout.
///
/// Errors if the process cannot be spawned or exits non-zero. A freshly-written
/// executable can transiently fail with `ETXTBSY` when another thread/process
/// forked while our write fd was still open (a well-known fork/exec race); we
/// retry briefly so concurrent task execution can't spuriously fail validation.
///
/// ```no_run
/// # // Reason: spawns a subprocess and performs filesystem/process IO.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn probe_binary_version(path: &Path) -> io::Result<String> {
    // ETXTBSY is 26 on both Linux and macOS; `ErrorKind::ExecutableFileBusy` is
    // not yet stable, so match the raw errno.
    const ETXTBSY: i32 = 26;
    let mut attempts = 0;
    loop {
        match Command::new(path).arg("--version").output() {
            Ok(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
            }
            Ok(output) => {
                return Err(io::Error::other(format!(
                    "`{} --version` exited with {}",
                    path.display(),
                    output.status,
                )));
            }
            Err(err) if err.raw_os_error() == Some(ETXTBSY) && attempts < 50 => {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(err) => return Err(err),
        }
    }
}
