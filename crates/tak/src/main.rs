//! `tak` binary entrypoint.
//!
//! The binary is intentionally thin and delegates command execution to the
//! library-backed CLI runtime in `tak::run_cli`.

use std::process::ExitCode;

/// Starts the Tak CLI process.
#[tokio::main]
async fn main() -> ExitCode {
    match tak::run_cli().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprint!("{}", tak::render_error_report(&err));
            ExitCode::FAILURE
        }
    }
}
