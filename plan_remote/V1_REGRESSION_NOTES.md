# V1 Regression Notes

This note tracks the current acceptance-criterion test ownership map.
Reference criteria: `plan_remote/V1_REFACTOR.md` §8.1-§8.13.

## Criterion -> Test -> Crate

| Criterion | Tests | Owning crate(s) |
| --- | --- | --- |
| §8.1 local-only execution | `run_local_only_execution_reports_local_placement_and_ignores_unused_remote_defs`, `executes_dependencies_before_target` | `crates/tak`, `crates/tak-exec` |
| §8.2 strict remote single | `run_remote_only_single_healthy_endpoint_reports_remote_placement`, `run_remote_only_single_unavailable_endpoint_fails_without_local_fallback`, `run_remote_only_single_legacy_reachable_endpoint_fails_without_local_fallback`, `remote_only_single_unavailable_endpoint_returns_infra_error`, `run_remote_only_handshake_follows_preflight_submit_events_result_order` | `crates/tak`, `crates/tak-exec` |
| §8.3 strict remote list fallback | `run_remote_only_list_falls_back_in_order_to_first_reachable_node`, `run_remote_only_list_stops_after_first_reachable_node`, `run_remote_only_list_all_unavailable_returns_infra_error_without_local_fallback`, `remote_only_list_falls_back_when_first_node_auth_rejects_submit` | `crates/tak`, `crates/tak-exec` |
| §8.4 policy runtime contract | `run_by_custom_policy_named_function_executes_runtime_policy_and_reports_reason`, `run_by_custom_policy_local_decision_uses_v1_context_surface_and_reports_reason`, `run_by_custom_policy_remote_decision_reports_node_reason_and_stays_stable_for_retries`, `evaluate_named_policy_decision_is_deterministic_for_identical_context_snapshot` | `crates/tak`, `crates/tak-loader`, `crates/tak-exec` |
| §8.5/§8.6 transfer boundary + deterministic include | `run_remote_only_current_state_boundary_is_deterministic`, `remote_execution_stages_only_current_state_manifest_files` | `crates/tak`, `crates/tak-exec` |
| §8.7 accepted transfer/result sync modes | `rejects_unsupported_remote_workspace_transfer_mode`, `rejects_unsupported_remote_result_sync_mode`, `remote_only_single_rejects_unsupported_result_sync_mode` | `crates/tak-loader`, `crates/tak-exec` |
| §8.8 no scoring model terms | `rejects_policy_decisions_with_scoring_fields`, `rejects_prefer_style_policy_api_calls`, `rejects_require_style_policy_api_calls` | `crates/tak-loader` |
| §8.9 lease behavior stability | `run_with_needs_acquires_and_releases_daemon_lease`, `run_waits_for_lease_then_releases_it`, `run_remote_task_with_needs_releases_lease_and_preserves_remote_metadata` | `crates/tak`, `crates/tak-exec` |
| §8.10 handshake lifecycle | `run_remote_only_handshake_follows_preflight_submit_events_result_order`, `run_remote_only_handshake_events_resume_uses_after_seq_without_duplicate_regression`, `serves_required_v1_endpoints_with_stable_contracts` | `crates/tak`, `crates/tak-exec`, `crates/takd` |
| §8.11 submit idempotency tuple | `sqlite_submit_idempotency_duplicate_attach_reuses_existing_attempt_state`, `sqlite_submit_idempotency_attempt_increment_creates_new_execution_scope`, `submit_endpoint_attaches_duplicate_attempt`, `key_changes_when_attempt_increments` | `crates/takd` |
| §8.12 Tor Arti onion parity | `run_remote_only_tor_onion_reaches_takd_with_embedded_arti_transport_parity`, `direct_and_tor_transports_share_remote_protocol_contract` | `crates/tak`, `crates/tak-exec`, `crates/takd` |
| §8.13 auth error handling | `remote_only_single_auth_rejection_returns_infra_auth_error`, `remote_only_single_auth_failure_during_capabilities_returns_infra_error`, `remote_only_list_falls_back_when_first_node_auth_rejects_submit`, `remote_only_list_all_auth_rejections_return_auth_infra_error` | `crates/tak-exec`, `crates/tak` |
