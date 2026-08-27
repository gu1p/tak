use std::collections::HashMap;

use super::FakeDockerRequest;

pub(super) fn requested_statuses(request: &FakeDockerRequest) -> Vec<String> {
    request
        .query_param("filters")
        .and_then(|filters| serde_json::from_str::<HashMap<String, Vec<String>>>(&filters).ok())
        .and_then(|mut filters| filters.remove("status"))
        .unwrap_or_default()
}
