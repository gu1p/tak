fn handle_remote_submit_route(
    store: &SubmitAttemptStore,
    body: Option<&str>,
) -> Result<RemoteV1Response> {
    let Some(body) = body else {
        return Ok(json_response(
            400,
            serde_json::json!({"accepted": false, "reason": "missing_body"}),
        ));
    };
    let payload: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => {
            return Ok(json_response(
                400,
                serde_json::json!({"accepted": false, "reason": "invalid_json"}),
            ));
        }
    };
    let task_run_id = payload
        .get("task_run_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let attempt = payload
        .get("attempt")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let selected_node_id = payload
        .get("selected_node_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();

    if task_run_id.is_empty() || attempt.is_none() || selected_node_id.is_empty() {
        return Ok(json_response(
            400,
            serde_json::json!({"accepted": false, "reason": "invalid_submit_fields"}),
        ));
    }

    let worker_payload = parse_remote_worker_submit_payload(&payload)?;
    let registration = store.register_submit(task_run_id, attempt, selected_node_id)?;
    let (attached, idempotency_key) = match registration {
        SubmitRegistration::Created { idempotency_key } => (false, idempotency_key),
        SubmitRegistration::Attached { idempotency_key } => (true, idempotency_key),
    };

    if !attached && let Some(worker_payload) = worker_payload.clone() {
        spawn_remote_worker_submit_execution(
            store.clone(),
            idempotency_key.clone(),
            selected_node_id.to_string(),
            worker_payload,
        );
    }

    let execution_mode = if worker_payload.is_some() {
        "remote_worker"
    } else {
        "store_only"
    };
    Ok(json_response(
        200,
        serde_json::json!({
            "accepted": true,
            "attached": attached,
            "idempotency_key": idempotency_key,
            "execution_mode": execution_mode,
            "remote_worker": worker_payload.is_some(),
        }),
    ))
}
