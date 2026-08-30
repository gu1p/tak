use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{Operation, Request, Response, RunLifecycleState};

pub(super) async fn run(socket_path: &Path, run_id: &str) -> Result<()> {
    let mut after_event = 0;
    loop {
        let response = super::exchange::response(
            socket_path,
            &Request {
                request_id: super::exchange::request_id("attach"),
                operation: Operation::AttachRun {
                    run_id: run_id.to_owned(),
                    after_event,
                },
            },
        )
        .await?;
        let Response::RunEvents {
            run_id: response_run,
            events,
            next_event,
            state,
            terminal,
            ..
        } = response
        else {
            bail!("local takd returned an unexpected AttachRun response")
        };
        crate::cli::runs_cli::attach::validate_event_page(
            run_id,
            &response_run,
            after_event,
            &events,
            next_event,
            state,
            terminal,
        )?;
        for event in &events {
            super::render::event(event);
        }
        after_event = next_event;
        if terminal {
            return match state {
                RunLifecycleState::Succeeded => Ok(()),
                RunLifecycleState::Failed | RunLifecycleState::Cancelled => {
                    bail!("run {run_id} did not succeed")
                }
                _ => bail!("local takd returned an invalid terminal run state"),
            };
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
