use std::io;
use std::path::Path;

use serde::Deserialize;

use super::CreateRecord;
use super::request::FakeDockerRequest;
use super::state::FakeDockerDaemonState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CreateContainerPayload {
    image: Option<String>,
    #[serde(default)]
    cmd: Vec<String>,
    working_dir: Option<String>,
    host_config: Option<HostConfigPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfigPayload {
    binds: Option<Vec<String>>,
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
    let binds = payload
        .host_config
        .and_then(|config| config.binds)
        .unwrap_or_default();
    let exit_code = exit_code_for_payload(state, &payload.cmd, &binds);
    Ok(CreatedContainer {
        record: CreateRecord {
            container_id,
            image: payload.image,
            cmd: payload.cmd,
            working_dir: payload.working_dir,
            binds,
        },
        exit_code,
    })
}

fn parse_create_payload(request: &FakeDockerRequest) -> io::Result<CreateContainerPayload> {
    serde_json::from_slice(&request.body)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn exit_code_for_payload(state: &FakeDockerDaemonState, cmd: &[String], binds: &[String]) -> i64 {
    let Some(bind_source) = binds
        .first()
        .and_then(|bind| bind.split(':').next())
        .map(Path::new)
    else {
        return 1;
    };
    let visible = state.path_is_visible(bind_source);
    let is_probe = cmd.iter().any(|value| value.contains(".tak-mount-visible"));

    if is_probe {
        let sentinel = bind_source.join(".tak-mount-visible");
        return if visible && sentinel.is_file() { 0 } else { 1 };
    }

    if cmd.iter().any(|value| value.contains("exit 1")) {
        return 1;
    }

    if visible { 0 } else { 1 }
}
