use std::collections::BTreeMap;
use std::io;

use super::BuildRecord;
use super::request::FakeDockerRequest;
use super::tar::{tar_file_entries, tar_file_modes};

pub(super) fn parse_build_request(request: &FakeDockerRequest) -> io::Result<BuildRecord> {
    Ok(BuildRecord {
        image_tag: request.query_param("t").unwrap_or_default(),
        dockerfile: request
            .query_param("dockerfile")
            .unwrap_or_else(|| "Dockerfile".to_string()),
        context_entries: tar_file_entries(&request.body)?,
        context_modes: tar_file_modes(&request.body)?
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
    })
}
