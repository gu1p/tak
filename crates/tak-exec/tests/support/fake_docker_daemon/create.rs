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
}

pub(super) fn parse_create_request(request: &FakeDockerRequest) -> io::Result<CreateRecord> {
    let payload: CreateContainerPayload = serde_json::from_slice(&request.body)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(CreateRecord {
        user: payload.user,
        binds: payload
            .host_config
            .and_then(|config| config.binds)
            .unwrap_or_default(),
    })
}
