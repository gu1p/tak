use std::collections::BTreeMap;

use tak_core::v2::{WorkspaceEntry, WorkspaceEntryType};
use tak_proto::worker_v2::{
    OutputChunkRequest, WorkerOutputArtifact, decode_output_chunk_response,
    encode_output_chunk_request,
};

use super::*;

const CHUNK_BYTES: u32 = 256 * 1024;

pub(super) async fn import(
    transport: &RemoteAttemptTransport,
    target: &WorkerConnectionTarget,
    command: &DispatchCommand,
    outputs: &[WorkerOutputArtifact],
) -> Result<()> {
    let acceptance = transport
        .store
        .prevalidate_remote_attempt_outputs(command, outputs)?;
    if acceptance == super::super::scheduler::ResultAcceptance::Stale {
        bail!("remote output attempt fence is stale");
    }
    let mut manifests = BTreeMap::<String, Vec<WorkspaceEntry>>::new();
    for output in outputs {
        if output.entry.entry_type == WorkspaceEntryType::File {
            let bytes = download(transport, target, command, output).await?;
            transport
                .store
                .import_remote_output_blob(&output.entry, &bytes)?;
        }
        manifests
            .entry(output.producer_task_id.clone())
            .or_default()
            .push(output.entry.clone());
    }
    for (producer, entries) in manifests {
        let accepted = transport
            .store
            .persist_attempt_task_outputs(command, &producer, &entries)?;
        if accepted == super::super::scheduler::ResultAcceptance::Stale {
            bail!("remote output attempt fence is stale");
        }
    }
    Ok(())
}

async fn download(
    transport: &RemoteAttemptTransport,
    target: &WorkerConnectionTarget,
    command: &DispatchCommand,
    output: &WorkerOutputArtifact,
) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        let request = OutputChunkRequest {
            protocol_version: 2,
            identity: request::identity(command),
            artifact_id: output.artifact_id.clone(),
            offset: bytes.len() as u64,
            max_bytes: CHUNK_BYTES,
        };
        let response = transport
            .broker
            .worker_v2_http_exchange(
                target,
                "POST",
                "/v2/attempts/output-chunk",
                &encode_output_chunk_request(&request)?,
            )
            .await?;
        require_status(response.status, &[200], "output download")?;
        let chunk = decode_output_chunk_response(&response.body, &request)?;
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            chunk.chunk_base64,
        )?;
        if decoded.is_empty() && !chunk.eof {
            bail!("worker output download made no progress")
        }
        bytes.extend_from_slice(&decoded);
        if bytes.len() as u64 > output.entry.size {
            bail!("worker output exceeded declared size")
        }
        if chunk.eof {
            break;
        }
    }
    if bytes.len() as u64 != output.entry.size {
        bail!("worker output ended before declared size")
    }
    Ok(bytes)
}
