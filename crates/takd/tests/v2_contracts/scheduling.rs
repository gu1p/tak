macro_rules! sibling_test_module {
    ($name:ident) => {
        mod $name {
            include!(concat!("../", stringify!($name), ".rs"));
        }
    };
}

sibling_test_module!(v2_scheduler_affinity_behavior);
sibling_test_module!(v2_scheduler_affinity_group_behavior);
sibling_test_module!(v2_scheduler_affinity_migration_conflict_contract);
sibling_test_module!(v2_scheduler_affinity_migration_contract);
sibling_test_module!(v2_scheduler_atomic_reservation_contract);
sibling_test_module!(v2_scheduler_backoff_contract);
sibling_test_module!(v2_scheduler_balanced_behavior);
sibling_test_module!(v2_scheduler_cache_reset_contract);
sibling_test_module!(v2_scheduler_worker_snapshot_accounting_behavior);
sibling_test_module!(v2_scheduler_cancellation_contract);
sibling_test_module!(v2_scheduler_constraint_cancellation_behavior);
sibling_test_module!(v2_scheduler_definition_conflict_behavior);
sibling_test_module!(v2_scheduler_definition_scope_conflict_behavior);
sibling_test_module!(v2_scheduler_exit_filter_contract);
sibling_test_module!(v2_scheduler_fairness_behavior);
sibling_test_module!(v2_scheduler_fencing_contract);
sibling_test_module!(v2_scheduler_first_available_behavior);
sibling_test_module!(v2_scheduler_limiter_behavior);
sibling_test_module!(v2_scheduler_limiter_kinds_behavior);
sibling_test_module!(v2_scheduler_live_inventory_filter_behavior);
sibling_test_module!(v2_scheduler_process_cap_behavior);
sibling_test_module!(v2_scheduler_remote_process_cap_behavior);
sibling_test_module!(v2_scheduler_node_loss_behavior);
sibling_test_module!(v2_scheduler_node_loss_retry_behavior);
sibling_test_module!(v2_scheduler_node_recovery_contract);
sibling_test_module!(v2_scheduler_queue_behavior);
sibling_test_module!(v2_scheduler_queue_priority_behavior);
sibling_test_module!(v2_scheduler_rate_atomic_behavior);
sibling_test_module!(v2_scheduler_rate_rollback_contract);
sibling_test_module!(v2_scheduler_readiness_contract);
sibling_test_module!(v2_scheduler_ready_age_contract);
sibling_test_module!(v2_scheduler_preaccept_retry_contract);
sibling_test_module!(v2_scheduler_retry_contract);
sibling_test_module!(v2_scheduler_retry_budget_contract);
sibling_test_module!(v2_scheduler_round_robin_behavior);
sibling_test_module!(v2_scheduler_scope_behavior);
sibling_test_module!(v2_scheduler_scope_key_behavior);
sibling_test_module!(v2_scheduler_shared_workspace_behavior);
sibling_test_module!(v2_scheduler_transport_persistence_contract);
sibling_test_module!(v2_scheduler_soft_affinity_behavior);
sibling_test_module!(v2_scheduler_soft_node_loss_behavior);
sibling_test_module!(v2_scheduler_worktree_queue_conflict_behavior);
sibling_test_module!(v2_scheduler_worktree_scope_behavior);
sibling_test_module!(v2_server_fused_output_attribution_behavior);
sibling_test_module!(v2_server_local_execution_behavior);
sibling_test_module!(v2_server_remote_execution_behavior);
