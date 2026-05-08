mod runtime_api;

use tak_core::model::TaskLabel;

#[test]
fn tak_exec_crate_root_public_api_stays_available() {
    let _run_tasks = tak_exec::run_tasks;
    let _run_resolved_task = tak_exec::run_resolved_task;
    let _default_client_tor_config = tak_exec::default_client_tor_config;
    let _load_remote_observation = tak_exec::load_remote_observation;
    let _load_remote_observation_at = tak_exec::load_remote_observation_at;
    let _record_remote_observation = tak_exec::record_remote_observation;
    let _write_remote_observation = tak_exec::write_remote_observation;
    let _write_remote_observation_at = tak_exec::write_remote_observation_at;
    let _target_set_from_summary = tak_exec::target_set_from_summary;
    let _endpoint_host_port = tak_exec::endpoint_host_port;
    let _endpoint_socket_addr = tak_exec::endpoint_socket_addr;
    let _socket_addr_from_host_port = tak_exec::socket_addr_from_host_port;

    let _run_options: tak_exec::RunOptions = Default::default();
    let _run_summary: tak_exec::RunSummary = Default::default();
    let _output_stream = tak_exec::OutputStream::Stdout;
    let _status_phase = tak_exec::TaskStatusPhase::RemoteProbe;
    let _placement_mode = tak_exec::PlacementMode::Local;
    let task_label = TaskLabel {
        package: "//".to_string(),
        name: "task".to_string(),
    };

    let _output_chunk = tak_exec::TaskOutputChunk {
        task_run_id: "task-run".to_string(),
        task_label: task_label.clone(),
        attempt: 1,
        stream: tak_exec::OutputStream::Stdout,
        bytes: Vec::new(),
    };
    let _status_event = tak_exec::TaskStatusEvent {
        task_label: task_label.clone(),
        attempt: 1,
        phase: tak_exec::TaskStatusPhase::RemoteWait,
        remote_node_id: None,
        message: String::new(),
    };
    let _task_started = tak_exec::TaskStartedEvent {
        task_run_id: "task-run".to_string(),
        task_label: task_label.clone(),
        placement_mode: tak_exec::PlacementMode::Local,
        remote_node_id: None,
        origin: None,
        runtime: None,
        runtime_source: None,
        command: None,
    };
    let _task_finished = tak_exec::TaskFinishedEvent {
        task_run_id: "task-run".to_string(),
        task_label: task_label.clone(),
        attempts: 1,
        success: true,
        exit_code: Some(0),
        placement_mode: tak_exec::PlacementMode::Local,
        remote_node_id: None,
    };
    let _remote_log = tak_exec::RemoteLogChunk {
        seq: 1,
        stream: tak_exec::OutputStream::Stderr,
        bytes: Vec::new(),
    };
    let _synced_output = tak_exec::SyncedOutput {
        path: "artifact.txt".to_string(),
        digest: "deadbeef".to_string(),
        size_bytes: 0,
    };
    let _task_result = tak_exec::TaskRunResult {
        task_run_id: "task-run".to_string(),
        attempts: 1,
        success: true,
        exit_code: Some(0),
        failure_detail: None,
        placement_mode: tak_exec::PlacementMode::Local,
        remote_node_id: None,
        remote_transport_kind: None,
        decision_reason: None,
        context_manifest_hash: None,
        remote_runtime_kind: None,
        remote_runtime_engine: None,
        session_name: None,
        session_reuse: None,
        remote_logs: Vec::new(),
        synced_outputs: Vec::new(),
    };
}
