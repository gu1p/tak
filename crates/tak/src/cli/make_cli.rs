use std::process::ExitCode;

use anyhow::{Context, Result};
use tak_make::{FilesystemMakefileReader, RunMake, RunMakeRequest};

use super::command_model::MakeArgs;

mod executor;
mod output;
mod task;

use executor::TakGoalExecutor;

pub(super) async fn run_make_command(args: MakeArgs) -> Result<ExitCode> {
    let workspace_root = std::env::current_dir().context("failed to resolve current directory")?;
    let goal = args.goal.clone();
    let executor = TakGoalExecutor::new(&args);
    let reader = FilesystemMakefileReader;
    let outcome = RunMake::new(&reader, &executor)
        .execute(RunMakeRequest {
            workspace_root,
            goal,
        })
        .await?;

    Ok(process_exit_code(outcome.exit_code))
}

fn process_exit_code(code: i32) -> ExitCode {
    ExitCode::from(code.clamp(0, u8::MAX as i32) as u8)
}
