use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(in crate::cli) enum RunsCommands {
    /// List daemon-owned graph runs.
    List,
    /// Show one daemon-owned graph run.
    Show {
        #[arg(value_name = "RUN_ID")]
        run_id: String,
    },
    /// Attach to one daemon-owned graph run.
    Attach {
        #[arg(value_name = "RUN_ID")]
        run_id: String,
    },
    /// Persist cancellation for one daemon-owned graph run.
    Cancel {
        #[arg(value_name = "RUN_ID")]
        run_id: String,
    },
    /// Retrieve final outputs without modifying the submitted checkout.
    Outputs {
        #[arg(value_name = "RUN_ID")]
        run_id: String,
        /// Destination directory for retrieved outputs.
        #[arg(long = "to", value_name = "DIR")]
        to: PathBuf,
    },
}
