use super::*;

pub(super) fn validate_target(
    request: &LocalBrokerRequest,
) -> std::result::Result<BrokerTarget, BrokerHttpError> {
    require_header(request, BROKER_VERSION_HEADER, "missing_broker_version").and_then(|value| {
        if value == "1" {
            Ok(())
        } else {
            Err(BrokerHttpError::bad_request("unsupported_broker_version"))
        }
    })?;
    let node_id = require_header(request, REMOTE_NODE_HEADER, "missing_remote_node")?.to_string();
    let endpoint =
        require_header(request, REMOTE_ENDPOINT_HEADER, "missing_remote_endpoint")?.to_string();
    let transport = require_header(request, REMOTE_TRANSPORT_HEADER, "missing_remote_transport")?;
    if transport != "tor" {
        return Err(BrokerHttpError::bad_request("unsupported_remote_transport"));
    }
    tak_core::endpoint::endpoint_host_port(&endpoint)
        .map_err(|_| BrokerHttpError::bad_request("invalid_remote_endpoint"))?;
    Ok(BrokerTarget { node_id, endpoint })
}

pub(super) fn validate_target_head(
    request: &LocalBrokerRequestHead,
) -> std::result::Result<BrokerTarget, BrokerHttpError> {
    require_header_head(request, BROKER_VERSION_HEADER, "missing_broker_version").and_then(
        |value| {
            if value == "1" {
                Ok(())
            } else {
                Err(BrokerHttpError::bad_request("unsupported_broker_version"))
            }
        },
    )?;
    let node_id =
        require_header_head(request, REMOTE_NODE_HEADER, "missing_remote_node")?.to_string();
    let endpoint = require_header_head(request, REMOTE_ENDPOINT_HEADER, "missing_remote_endpoint")?
        .to_string();
    let transport =
        require_header_head(request, REMOTE_TRANSPORT_HEADER, "missing_remote_transport")?;
    if transport != "tor" {
        return Err(BrokerHttpError::bad_request("unsupported_remote_transport"));
    }
    tak_core::endpoint::endpoint_host_port(&endpoint)
        .map_err(|_| BrokerHttpError::bad_request("invalid_remote_endpoint"))?;
    Ok(BrokerTarget { node_id, endpoint })
}

pub(super) fn prefers_http2(request: &LocalBrokerRequest) -> bool {
    request
        .header(REMOTE_PROTOCOL_HEADER)
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case("h2"))
}

pub(super) fn prefers_http2_head(request: &LocalBrokerRequestHead) -> bool {
    request
        .header(REMOTE_PROTOCOL_HEADER)
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case("h2"))
}

fn require_header<'a>(
    request: &'a LocalBrokerRequest,
    name: &str,
    code: &'static str,
) -> std::result::Result<&'a str, BrokerHttpError> {
    request
        .header(name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BrokerHttpError::bad_request(code))
}

fn require_header_head<'a>(
    request: &'a LocalBrokerRequestHead,
    name: &str,
    code: &'static str,
) -> std::result::Result<&'a str, BrokerHttpError> {
    request
        .header(name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| BrokerHttpError::bad_request(code))
}

pub(super) struct BrokerTarget {
    #[allow(dead_code)]
    pub(super) node_id: String,
    pub(super) endpoint: String,
}
