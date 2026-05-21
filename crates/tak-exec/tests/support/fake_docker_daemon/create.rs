use std::io;

use serde::Deserialize;

use super::CreateRecord;
use super::request::FakeDockerRequest;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CreateContainerPayload {
    user: Option<String>,
    host_config: Option<HostConfigPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfigPayload {
    binds: Option<Vec<String>>,
    nano_cpus: Option<i64>,
    memory: Option<i64>,
}

pub(super) fn parse_create_request(request: &FakeDockerRequest) -> io::Result<CreateRecord> {
    let payload: CreateContainerPayload = serde_json::from_slice(&request.body)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let host_config = payload.host_config;
    Ok(CreateRecord {
        user: payload.user,
        binds: host_config
            .as_ref()
            .and_then(|config| config.binds.clone())
            .unwrap_or_default(),
        nano_cpus: host_config.as_ref().and_then(|config| config.nano_cpus),
        memory: host_config.and_then(|config| config.memory),
    })
}
