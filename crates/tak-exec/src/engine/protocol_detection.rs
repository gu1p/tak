/// Probes whether the remote endpoint supports the V1 handshake preflight contract.
///
/// Unsupported or legacy endpoints silently degrade to reachability-only behavior.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn detect_remote_protocol_mode(target: &StrictRemoteTarget) -> Result<RemoteProtocolMode> {
    let capabilities = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/capabilities",
        None,
        "capabilities",
        Duration::from_millis(150),
    )
    .await;

    let (status, body) = match capabilities {
        Ok(response) => response,
        Err(err) => {
            if is_auth_configuration_failure(&err) {
                return Err(err);
            }
            return Ok(RemoteProtocolMode::LegacyReachability);
        }
    };

    if status == 401 || status == 403 {
        bail!(
            "infra error: remote node {} auth failed during capabilities with HTTP {}",
            target.node_id,
            status
        );
    }
    if status != 200 {
        return Ok(RemoteProtocolMode::LegacyReachability);
    }

    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(value) => value,
        Err(_) => return Ok(RemoteProtocolMode::LegacyReachability),
    };
    let Some(compatible) = parsed
        .get("compatible")
        .and_then(serde_json::Value::as_bool)
    else {
        return Ok(RemoteProtocolMode::LegacyReachability);
    };

    if !compatible {
        bail!(
            "infra error: remote node {} capability mismatch at {}",
            target.node_id,
            target.endpoint
        );
    }

    let status_response = remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/status",
        None,
        "status",
        Duration::from_millis(150),
    )
    .await;
    let (status_code, status_body) = match status_response {
        Ok(response) => response,
        Err(err) => {
            if is_auth_configuration_failure(&err) {
                return Err(err);
            }
            return Ok(RemoteProtocolMode::LegacyReachability);
        }
    };
    if status_code == 401 || status_code == 403 {
        bail!(
            "infra error: remote node {} auth failed during status with HTTP {}",
            target.node_id,
            status_code
        );
    }
    if status_code != 200 {
        return Ok(RemoteProtocolMode::LegacyReachability);
    }
    if let Ok(parsed_status) = serde_json::from_str::<serde_json::Value>(&status_body)
        && let Some(healthy) = parsed_status
            .get("healthy")
            .and_then(serde_json::Value::as_bool)
        && !healthy
    {
        bail!(
            "infra error: remote node {} reported unhealthy status at {}",
            target.node_id,
            target.endpoint
        );
    }

    let remote_worker = parsed
        .get("remote_worker")
        .and_then(serde_json::Value::as_bool)
        .or_else(|| {
            parsed
                .get("execution_mode")
                .and_then(serde_json::Value::as_str)
                .map(|mode| mode == "remote_worker")
        })
        .unwrap_or(false);

    Ok(RemoteProtocolMode::HandshakeV1 { remote_worker })
}
