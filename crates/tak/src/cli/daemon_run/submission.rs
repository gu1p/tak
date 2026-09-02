use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_core::v2::{PlacementCandidate, RemoteRequirements, RunSubmission};
use tak_proto::local_daemon::v2::{Operation, Request, Response, WorkspaceDisposition};

mod attach;
mod exchange;
mod render;
mod upload;

pub(super) async fn foreground_response(
    socket_path: &std::path::Path,
    request: &Request,
) -> Result<Response> {
    exchange::response(socket_path, request).await
}

pub(super) async fn remote_candidates(
    socket_path: &std::path::Path,
    requirements: RemoteRequirements,
) -> Result<Vec<PlacementCandidate>> {
    let response = exchange::response(
        socket_path,
        &Request {
            request_id: exchange::request_id("candidates"),
            operation: Operation::ResolveRemoteCandidates { requirements },
        },
    )
    .await?;
    let Response::RemoteCandidates { candidates, .. } = response else {
        bail!("local takd returned an unexpected RemoteCandidates response")
    };
    Ok(candidates)
}

pub(super) async fn submit_and_attach(
    socket_path: PathBuf,
    submission: RunSubmission,
    archive: Vec<u8>,
    checkout: crate::cli::run_checkout_store::CheckoutContext,
    renderer: Option<&dyn super::PersistedEventRenderer>,
) -> Result<ExitCode> {
    let mut interrupts = crate::cli::attachment_interrupt::State::new()?;
    let response = exchange::response(
        &socket_path,
        &Request {
            request_id: exchange::request_id("submit"),
            operation: Operation::SubmitRun {
                idempotency_key: submission.idempotency_key.clone(),
                run: Box::new(submission.run.clone()),
                environment_values: submission.environment_values.clone(),
            },
        },
    )
    .await?;
    let Response::RunSubmitted {
        run_id, workspace, ..
    } = response
    else {
        bail!("local takd returned an unexpected SubmitRun response")
    };
    println!("run_id={run_id}");
    crate::cli::run_checkout_store::RunCheckoutStore::open_default()?.record(
        &socket_path,
        &run_id,
        &checkout,
    )?;
    if let WorkspaceDisposition::UploadRequired { next_offset } = workspace {
        let upload = upload::workspace(
            &socket_path,
            &run_id,
            &submission.run.workspace.manifest.fingerprint,
            &archive,
            next_offset,
        );
        tokio::select! {
            result = upload => result?,
            action = interrupts.next() => {
                handle_pre_attach_interrupt(
                    &socket_path, &run_id, action?, &mut interrupts,
                ).await?;
                return attach::run_with_interrupts(
                    &socket_path, &run_id, interrupts, &checkout, renderer,
                ).await;
            }
        }
    }
    let commit_request = Request {
        request_id: exchange::request_id("commit"),
        operation: Operation::CommitRun {
            run_id: run_id.clone(),
        },
    };
    let commit = exchange::response(&socket_path, &commit_request);
    let response = tokio::select! {
        response = commit => response?,
        action = interrupts.next() => {
            handle_pre_attach_interrupt(
                &socket_path, &run_id, action?, &mut interrupts,
            ).await?;
            return attach::run_with_interrupts(
                &socket_path, &run_id, interrupts, &checkout, renderer,
            ).await;
        }
    };
    if !matches!(response, Response::RunCommitted { run_id: ref id, .. } if id == &run_id) {
        bail!("local takd returned an unexpected CommitRun response");
    }
    attach::run_with_interrupts(&socket_path, &run_id, interrupts, &checkout, renderer).await
}

async fn handle_pre_attach_interrupt(
    socket_path: &std::path::Path,
    run_id: &str,
    action: crate::cli::attachment_interrupt::Action,
    interrupts: &mut crate::cli::attachment_interrupt::State,
) -> Result<()> {
    if attach::handle_interrupt(socket_path, run_id, action, interrupts).await? {
        bail!("detached from run {run_id}; persisted cancellation continues")
    }
    Ok(())
}
