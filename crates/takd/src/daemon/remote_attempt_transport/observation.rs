use tak_proto::worker_v2::{
    AckAttemptRequest, CancelAttemptRequest, CancelDisposition, ObserveAttemptRequest,
    WorkerAttemptState, WorkerTerminalOutcome, decode_ack_response, decode_cancel_response,
    decode_observe_response_page, encode_ack_request, encode_cancel_request,
    encode_observe_request,
};

use super::super::scheduler::{AttemptCompletion, AttemptRuntimeMetadata};
use super::*;

pub(super) async fn reconcile(
    transport: &RemoteAttemptTransport,
    command: &DispatchCommand,
) -> Result<AttemptObservation> {
    let cursor = transport
        .store
        .worker_event_cursor(command)?
        .ok_or_else(|| anyhow::anyhow!("remote attempt is no longer current"))?;
    let target = transport.target(command)?;
    let request = ObserveAttemptRequest {
        protocol_version: 2,
        identity: request::identity(command),
        after_event: cursor,
    };
    let response = transport
        .broker
        .worker_v2_http_exchange(
            &target,
            "POST",
            "/v2/attempts/observe",
            &encode_observe_request(&request)?,
        )
        .await?;
    require_status(response.status, &[200], "observation")?;
    let observed = decode_observe_response_page(&response.body, &command.fencing_token, cursor)?;
    let events = transport.store.ingest_worker_events(
        command,
        cursor,
        &observed.events,
        observed.next_event,
    )?;
    if events == super::super::scheduler::ResultAcceptance::Stale {
        bail!("remote worker events arrived after their attempt fence closed");
    }
    match observed.state {
        WorkerAttemptState::Running => Ok(AttemptObservation::Running),
        WorkerAttemptState::Missing => Ok(AttemptObservation::Missing),
        WorkerAttemptState::Completed => {
            let terminal = observed.terminal.expect("validated completed observation");
            if terminal.outcome == WorkerTerminalOutcome::Succeeded {
                let acceptance = transport.store.begin_output_commit(command)?;
                if acceptance == super::super::scheduler::ResultAcceptance::Stale {
                    bail!("remote worker outputs arrived after their attempt fence closed");
                }
                outputs::import(transport, &target, command, &terminal.outputs).await?;
            }
            let runtime = terminal
                .runtime_kind
                .zip(terminal.runtime_engine)
                .map(|(kind, engine)| AttemptRuntimeMetadata { kind, engine });
            let completion = if terminal.outcome == WorkerTerminalOutcome::Succeeded {
                AttemptCompletion::Succeeded {
                    terminal_digest: terminal.terminal_digest,
                }
            } else {
                AttemptCompletion::Failed {
                    terminal_digest: terminal.terminal_digest,
                    exit_code: terminal.exit_code,
                }
            }
            .with_runtime(runtime);
            Ok(AttemptObservation::Completed(completion))
        }
    }
}

pub(super) async fn cancel(
    transport: &RemoteAttemptTransport,
    command: &DispatchCommand,
) -> Result<()> {
    let target = transport.target(command)?;
    let request = CancelAttemptRequest {
        protocol_version: 2,
        identity: request::identity(command),
    };
    let response = transport
        .broker
        .worker_v2_http_exchange(
            &target,
            "POST",
            "/v2/attempts/cancel",
            &encode_cancel_request(&request)?,
        )
        .await?;
    require_status(response.status, &[200, 202], "cancellation")?;
    let response = decode_cancel_response(&response.body, &command.fencing_token)?;
    match response.disposition {
        CancelDisposition::Requested | CancelDisposition::Duplicate => {
            wait_until_stopped(transport, command).await
        }
        CancelDisposition::AlreadyTerminal | CancelDisposition::Stale => Ok(()),
    }
}

async fn wait_until_stopped(
    transport: &RemoteAttemptTransport,
    command: &DispatchCommand,
) -> Result<()> {
    let target = transport.target(command)?;
    let mut after_event = 0;
    loop {
        let request = ObserveAttemptRequest {
            protocol_version: 2,
            identity: request::identity(command),
            after_event,
        };
        let response = transport
            .broker
            .worker_v2_http_exchange(
                &target,
                "POST",
                "/v2/attempts/observe",
                &encode_observe_request(&request)?,
            )
            .await?;
        require_status(response.status, &[200], "cancellation observation")?;
        let observed =
            decode_observe_response_page(&response.body, &command.fencing_token, after_event)?;
        after_event = observed.next_event;
        match observed.state {
            WorkerAttemptState::Running => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            WorkerAttemptState::Completed | WorkerAttemptState::Missing => return Ok(()),
        }
    }
}

pub(super) async fn acknowledge(
    transport: &RemoteAttemptTransport,
    command: &DispatchCommand,
    terminal_digest: &str,
    run_terminal: bool,
) -> Result<()> {
    let target = transport.target(command)?;
    let request = AckAttemptRequest {
        protocol_version: 2,
        identity: request::identity(command),
        terminal_digest: terminal_digest.to_owned(),
        run_terminal,
    };
    let response = transport
        .broker
        .worker_v2_http_exchange(
            &target,
            "POST",
            "/v2/attempts/ack",
            &encode_ack_request(&request)?,
        )
        .await?;
    require_status(response.status, &[200], "terminal acknowledgement")?;
    decode_ack_response(&response.body, &request)?;
    Ok(())
}
