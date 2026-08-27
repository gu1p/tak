use std::io;
use std::path::Path;

use serde::Deserialize;
use std::collections::BTreeMap;

use super::CreateRecord;
use super::request::FakeDockerRequest;
use super::state::FakeDockerDaemonState;

mod exit_code;

use exit_code::exit_code_for_payload;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CreateContainerPayload {
    image: Option<String>,
    #[serde(default)]
    cmd: Vec<String>,
    user: Option<String>,
    working_dir: Option<String>,
    labels: Option<BTreeMap<String, String>>,
    #[serde(default)]
    env: Vec<String>,
    host_config: Option<HostConfigPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfigPayload {
    binds: Option<Vec<String>>,
    nano_cpus: Option<i64>,
}

pub(super) struct CreatedContainer {
    pub(super) record: CreateRecord,
    pub(super) exit_code: i64,
}

pub(super) fn create_container(
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<CreatedContainer> {
    let payload = parse_create_payload(request)?;
    let container_id = state.next_container_id();
    let (binds, nano_cpus) = payload.host_config.map_or((Vec::new(), None), |config| {
        (config.binds.unwrap_or_default(), config.nano_cpus)
    });
    let exit_code = exit_code_for_payload(state, &payload.cmd, &binds);
    Ok(CreatedContainer {
        record: CreateRecord {
            container_id,
            image: payload.image,
            cmd: payload.cmd,
            user: payload.user,
            working_dir: payload.working_dir,
            binds,
            labels: payload.labels.unwrap_or_default(),
            env: payload.env,
            nano_cpus,
            state: "running".to_string(),
        },
        exit_code,
    })
}

fn parse_create_payload(request: &FakeDockerRequest) -> io::Result<CreateContainerPayload> {
    serde_json::from_slice(&request.body)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}
