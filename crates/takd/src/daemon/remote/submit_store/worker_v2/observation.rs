use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use tak_proto::worker_v2::{
    MAX_OBSERVE_EVENTS, ObserveAttemptResponse, WorkerAttemptEvent, WorkerAttemptIdentity,
    decode_observe_response, encode_observe_response,
};

use super::{SubmitAttemptStore, protocol_state};

impl SubmitAttemptStore {
    pub fn observe_worker_v2_attempt(
        &self,
        identity: &WorkerAttemptIdentity,
        after_event: u64,
    ) -> Result<ObserveAttemptResponse> {
        let connection = self.open_connection()?;
        let record = connection
            .query_row(
                "SELECT state,terminal_json FROM worker_v2_attempts WHERE run_id=?1 AND \
                 job_id=?2 AND authored_attempt=?3 AND dispatch_generation=?4 AND \
                 fencing_token=?5 AND node_id=?6",
                params![
                    identity.run_id,
                    identity.job_id,
                    identity.authored_attempt,
                    identity.dispatch_generation,
                    identity.fencing_token,
                    identity.node_id
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        let Some((state, terminal_json)) = record else {
            return missing(identity, after_event);
        };
        let (events, has_more) = events_after(&connection, &identity.fencing_token, after_event)?;
        let next_event = events.last().map_or(after_event, |event| event.seq);
        if events
            .first()
            .is_some_and(|event| event.seq != after_event + 1)
        {
            anyhow::bail!("worker event sequence contains a gap");
        }
        let response = ObserveAttemptResponse {
            protocol_version: 2,
            fencing_token: identity.fencing_token.clone(),
            state: if has_more {
                tak_proto::worker_v2::WorkerAttemptState::Running
            } else {
                protocol_state(&state)?
            },
            events,
            next_event,
            terminal: if has_more {
                None
            } else {
                terminal_json
                    .as_deref()
                    .map(serde_json::from_str)
                    .transpose()?
            },
        };
        decode_observe_response(
            &encode_observe_response(&response)?,
            &identity.fencing_token,
        )
    }
}

fn missing(identity: &WorkerAttemptIdentity, after_event: u64) -> Result<ObserveAttemptResponse> {
    let response = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: identity.fencing_token.clone(),
        state: tak_proto::worker_v2::WorkerAttemptState::Missing,
        events: vec![],
        next_event: after_event,
        terminal: None,
    };
    decode_observe_response(
        &encode_observe_response(&response)?,
        &identity.fencing_token,
    )
}

fn events_after(
    connection: &rusqlite::Connection,
    fence: &str,
    after_event: u64,
) -> Result<(Vec<WorkerAttemptEvent>, bool)> {
    let after_event = i64::try_from(after_event)?;
    let mut statement = connection.prepare(
        "SELECT event_json FROM worker_v2_events WHERE fencing_token=?1 AND seq>?2 ORDER BY seq \
         LIMIT ?3",
    )?;
    let mut events = statement
        .query_map(
            params![fence, after_event, i64::try_from(MAX_OBSERVE_EVENTS + 1)?],
            |row| row.get::<_, String>(0),
        )?
        .map(|encoded| Ok(serde_json::from_str(&encoded?)?))
        .collect::<Result<Vec<_>>>()?;
    let has_more = events.len() > MAX_OBSERVE_EVENTS;
    events.truncate(MAX_OBSERVE_EVENTS);
    Ok((events, has_more))
}
