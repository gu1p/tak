use std::io;

use tokio::net::UnixStream;

use super::response::write_response;
use super::state::FakeDockerDaemonState;

pub(super) async fn write_stats_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
) -> io::Result<()> {
    let cpu = serde_json::json!({
        "cpu_usage": {
            "percpu_usage": [0],
            "usage_in_usermode": 0,
            "total_usage": 0,
            "usage_in_kernelmode": 0
        },
        "system_cpu_usage": 0,
        "online_cpus": 1,
        "throttling_data": {"periods": 0, "throttled_periods": 0, "throttled_time": 0}
    });
    let body = serde_json::json!({
        "read": "",
        "preread": "",
        "num_procs": 1,
        "pids_stats": {},
        "memory_stats": {"usage": state.memory_usage_bytes},
        "blkio_stats": {},
        "cpu_stats": cpu.clone(),
        "precpu_stats": cpu,
        "storage_stats": {},
        "name": "/fake",
        "id": "fake"
    });
    write_response(
        stream,
        "200 OK",
        "application/json",
        serde_json::to_string(&body)
            .expect("serialize fake Docker stats")
            .as_bytes(),
    )
    .await
}
