use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use tak_core::v2::{PlacementCandidate, RemoteRequirements, RunSubmission};
use tak_proto::local_daemon::v2::{Operation, Request, Response, WorkspaceDisposition};

mod attach;
#[cfg(test)]
mod dashboard_fallback_bdd_tests;
#[cfg(test)]
mod dashboard_fallback_tests;
mod exchange;
#[cfg(test)]
mod exchange_tests;
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
    let mut dashboard = crate::cli::run_dashboard::start_or_disable(
        crate::cli::run_dashboard::RunDashboard::detect(
            crate::cli::run_dashboard::DashboardSeed::from((&*run_id, &submission.run)),
        ),
        "before workspace upload",
    );
    set_renderer_dashboard(renderer, dashboard.is_some());
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
        tokio::pin!(upload);
        loop {
            tokio::select! {
                result = &mut upload => { result?; break; }
                action = interrupts.next() => {
                    handle_pre_attach_interrupt(
                        &socket_path, &run_id, action?, &mut interrupts, &mut dashboard, renderer,
                    ).await?;
                    return attach::run_with_interrupts(
                        &socket_path, &run_id, interrupts, &checkout, renderer,
                        &mut dashboard,
                    ).await;
                }
                input = attach::next_dashboard_interrupt(dashboard.as_mut()) => {
                    if let Err(error) = input {
                        crate::cli::run_dashboard::disable_after_error(
                            &mut dashboard, error, "during workspace upload",
                        );
                        set_renderer_dashboard(renderer, false);
                        continue;
                    }
                    handle_pre_attach_interrupt(
                        &socket_path, &run_id, interrupts.record(), &mut interrupts,
                        &mut dashboard, renderer,
                    ).await?;
                    return attach::run_with_interrupts(
                        &socket_path, &run_id, interrupts, &checkout, renderer,
                        &mut dashboard,
                    ).await;
                }
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
    tokio::pin!(commit);
    let response = loop {
        tokio::select! {
            response = &mut commit => break response?,
            action = interrupts.next() => {
                handle_pre_attach_interrupt(
                    &socket_path, &run_id, action?, &mut interrupts, &mut dashboard, renderer,
                ).await?;
                return attach::run_with_interrupts(
                    &socket_path, &run_id, interrupts, &checkout, renderer,
                    &mut dashboard,
                ).await;
            }
            input = attach::next_dashboard_interrupt(dashboard.as_mut()) => {
                if let Err(error) = input {
                    crate::cli::run_dashboard::disable_after_error(
                        &mut dashboard, error, "during run commit",
                    );
                    set_renderer_dashboard(renderer, false);
                    continue;
                }
                handle_pre_attach_interrupt(
                    &socket_path, &run_id, interrupts.record(), &mut interrupts,
                    &mut dashboard, renderer,
                ).await?;
                return attach::run_with_interrupts(
                    &socket_path, &run_id, interrupts, &checkout, renderer,
                    &mut dashboard,
                ).await;
            }
        }
    };
    if !matches!(response, Response::RunCommitted { run_id: ref id, .. } if id == &run_id) {
        bail!("local takd returned an unexpected CommitRun response");
    }
    attach::run_with_interrupts(
        &socket_path,
        &run_id,
        interrupts,
        &checkout,
        renderer,
        &mut dashboard,
    )
    .await
}

fn set_renderer_dashboard(renderer: Option<&dyn super::PersistedEventRenderer>, active: bool) {
    if let Some(renderer) = renderer {
        renderer.set_dashboard_active(active);
    }
}

async fn handle_pre_attach_interrupt(
    socket_path: &std::path::Path,
    run_id: &str,
    action: crate::cli::attachment_interrupt::Action,
    interrupts: &mut crate::cli::attachment_interrupt::State,
    dashboard: &mut Option<crate::cli::run_dashboard::RunDashboard>,
    renderer: Option<&dyn super::PersistedEventRenderer>,
) -> Result<()> {
    if attach::handle_interrupt(socket_path, run_id, action, interrupts, dashboard, renderer)
        .await?
    {
        bail!("detached from run {run_id}; persisted cancellation continues")
    }
    Ok(())
}
