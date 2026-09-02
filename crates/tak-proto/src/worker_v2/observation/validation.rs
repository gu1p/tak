use anyhow::{Result, bail};
use base64::Engine;
use sha2::{Digest, Sha256};

use super::{
    DispatchAttemptResponse, ObserveAttemptResponse, WorkerAttemptEvent, WorkerAttemptState,
};
use crate::worker_v2::PROTOCOL_VERSION;

mod outputs;

const MAX_EVENT_BYTES: usize = 64 * 1024;

pub(super) fn validate_dispatch(response: &DispatchAttemptResponse, fence: &str) -> Result<()> {
    validate_envelope(response.protocol_version, &response.fencing_token, fence)
}

pub(super) fn validate_observation(response: &ObserveAttemptResponse, fence: &str) -> Result<()> {
    validate_envelope(response.protocol_version, &response.fencing_token, fence)?;
    let events_are_ordered = response.events.len() <= super::MAX_OBSERVE_EVENTS
        && response.events.iter().all(validate_event)
        && response
            .events
            .windows(2)
            .all(|pair| pair[0].seq < pair[1].seq)
        && response
            .events
            .last()
            .is_none_or(|event| event.seq == response.next_event);
    if !events_are_ordered {
        bail!("worker attempt events are invalid");
    }
    match (&response.state, &response.terminal) {
        (WorkerAttemptState::Completed, Some(terminal)) => {
            outputs::validate_terminal(terminal, response.next_event)
        }
        (WorkerAttemptState::Running, None) => Ok(()),
        (WorkerAttemptState::Missing, None) => Ok(()),
        _ => bail!("worker attempt state and terminal are inconsistent"),
    }
}

pub(super) fn validate_observation_page(
    response: &ObserveAttemptResponse,
    after_event: u64,
) -> Result<()> {
    let mut expected = after_event;
    for event in &response.events {
        expected = expected
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("worker event cursor overflow"))?;
        if event.seq != expected {
            bail!("worker event page does not resume at the requested cursor");
        }
    }
    if response.next_event != expected {
        bail!("worker event page does not resume at the requested cursor");
    }
    Ok(())
}

fn validate_envelope(version: u16, actual_fence: &str, expected_fence: &str) -> Result<()> {
    if version != PROTOCOL_VERSION {
        bail!("worker protocol v2 is required; upgrade tak, takd, and workers together");
    }
    if !valid_identifier(actual_fence) || actual_fence != expected_fence {
        bail!("worker response fencing token mismatch");
    }
    Ok(())
}

fn validate_event(event: &WorkerAttemptEvent) -> bool {
    let Ok(chunk) = base64::engine::general_purpose::STANDARD.decode(&event.chunk_base64) else {
        return false;
    };
    event.seq > 0
        && valid_identifier(&event.task_id)
        && valid_digest(&event.chunk_sha256)
        && chunk.len() <= MAX_EVENT_BYTES
        && format!("{:x}", Sha256::digest(chunk)) == event.chunk_sha256
}

pub(super) fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

pub(super) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
