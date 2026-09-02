mod runtime_api;

use tak_core::model::TaskLabel;

#[test]
fn passive_diagnostic_and_output_api_stays_available() {
    let _endpoint_host_port = tak_exec::endpoint_host_port;
    let _endpoint_socket_addr = tak_exec::endpoint_socket_addr;
    let _socket_addr_from_host_port = tak_exec::socket_addr_from_host_port;

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
    let _task_finished = tak_exec::TaskFinishedEvent {
        task_run_id: "task-run".to_string(),
        task_label,
        attempts: 1,
        success: true,
        exit_code: Some(0),
        placement_mode: tak_exec::PlacementMode::Local,
        remote_node_id: None,
    };
}
