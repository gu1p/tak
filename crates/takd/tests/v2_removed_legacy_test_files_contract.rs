use std::path::Path;

#[test]
fn unreachable_v1_worker_and_executor_contracts_stay_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tests = root.join("tests");
    for removed in [
        "remote_binary_contract.rs",
        "remote_cancel_active_behavior.rs",
        "remote_cancel_behavior.rs",
        "remote_cleanup_janitor_behavior.rs",
        "remote_cleanup_janitor_isolation_behavior.rs",
        "remote_cleanup_shared_session_behavior.rs",
        "remote_container_cleanup_result_behavior.rs",
        "remote_container_infrastructure_failure_behavior.rs",
        "remote_container_janitor_behavior.rs",
        "remote_container_nonzero_exit_behavior.rs",
        "remote_container_user_behavior.rs",
        "remote_container_user_support.rs",
        "remote_contracts.rs",
        "remote_emergency_capacity_behavior.rs",
        "remote_exec_root_explicit_behavior.rs",
        "remote_exec_root_fallback_behavior.rs",
        "remote_exec_root_probe_arm64_behavior.rs",
        "remote_exec_root_probe_behavior.rs",
        "remote_exec_root_probe_retry_behavior.rs",
        "remote_exec_root_probe_unknown_arch_behavior.rs",
        "remote_exec_root_probe_version_error_behavior.rs",
        "remote_exec_root_simulated_runtime_behavior.rs",
        "remote_fused_container_oom_retry_behavior.rs",
        "remote_memory_pressure_recovery_behavior.rs",
        "remote_memory_pressure_recovery_support.rs",
        "remote_node_log_tail_bounded_contract.rs",
        "remote_node_logs_contract.rs",
        "remote_orphan_watchdog_behavior.rs",
        "remote_output_legacy_root_behavior.rs",
        "remote_output_query_decode_behavior.rs",
        "remote_output_query_reserved_character_behavior.rs",
        "remote_output_range_resume_contract.rs",
        "remote_resource_admission_behavior.rs",
        "remote_session_contract.rs",
        "remote_session_key_validation_behavior.rs",
        "remote_shared_docker_node_isolation_behavior.rs",
        "remote_status_behavior.rs",
        "remote_status_unavailable_behavior.rs",
        "remote_streaming_binary_contract.rs",
        "remote_streaming_chunk_bytes_contract.rs",
        "remote_submit_runtime_required_behavior.rs",
        "remote_v1_http2_server_contract.rs",
        "remote_v1_http_request_parse_behavior.rs",
        "remote_v1_http_request_validation_behavior.rs",
        "remote_v1_http_server_behavior.rs",
        "remote_v1_http_server_tor_behavior.rs",
        "remote_v1_http_truncated_submit_behavior.rs",
        "remote_worker_timing_behavior.rs",
        "support/remote_binary.rs",
        "support/remote_binary/events.rs",
        "support/remote_binary/runtime.rs",
        "support/remote_binary/submission.rs",
        "support/remote_container/command.rs",
        "support/remote_container/fused.rs",
        "support/remote_container/result.rs",
        "support/remote_output/runtime.rs",
        "support/remote_output/submit.rs",
        "support/remote_session.rs",
        "support/wait_for_session_task.rs",
        "support/wait_for_terminal_events.rs",
        "task_list_contract/live.rs",
        "workspace_wormhole_upload_contract.rs",
    ] {
        assert!(
            !tests.join(removed).exists(),
            "legacy test remains: {removed}"
        );
    }

    for retained in ["support/remote_container.rs", "support/remote_output.rs"] {
        let source = std::fs::read_to_string(tests.join(retained)).expect(retained);
        for legacy in ["/v1/", "SubmitTask", "GetTaskResult", "CancelTask"] {
            assert!(
                !source.contains(legacy),
                "legacy execution support remains in {retained}: {legacy}"
            );
        }
    }

    let parser = std::fs::read_to_string(
        root.join("src/daemon/remote/http_server_request_validation_unit_tests.rs"),
    )
    .unwrap();
    assert!(!parser.contains("/v1/"));
    assert!(parser.contains("/v2/worker/identity"));
}
