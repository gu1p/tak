use std::thread;
use std::time::Duration;

use base64::Engine;
use serde_json::Value;
use takd::{SubmitAttemptStore, handle_remote_v1_request};

fn build_zip_base64() -> String {
    let mut writer = std::io::Cursor::new(Vec::<u8>::new());
    {
        let mut zip = zip::ZipWriter::new(&mut writer);
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("input.txt", options)
            .expect("start zip file");
        std::io::Write::write_all(&mut zip, b"hello-from-zip\n").expect("write zip data");
        zip.finish().expect("finish zip archive");
    }
    base64::engine::general_purpose::STANDARD.encode(writer.into_inner())
}

#[test]
fn submit_with_inline_workspace_zip_executes_and_serves_output_download() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");
    let exec_root = temp.path().join("remote-exec");
    unsafe {
        std::env::set_var("TAKD_REMOTE_EXEC_ROOT", &exec_root);
    }

    let payload = serde_json::json!({
        "task_run_id": "worker-run-1",
        "attempt": 1,
        "task_label": "apps/web:remote_worker",
        "selected_node_id": "remote-node-a",
        "workspace": {
            "mode": "REPO_ZIP_SNAPSHOT",
            "archive_zip_base64": build_zip_base64(),
            "manifest_hash": "manifest-test-hash",
        },
        "execution": {
            "steps": [
                {
                    "kind": "cmd",
                    "argv": ["sh", "-c", "mkdir -p dist && cat input.txt > dist/out.txt"],
                    "env": {}
                }
            ],
            "timeout_s": 5
        },
        "result": {
            "sync_mode": "OUTPUTS_AND_LOGS"
        }
    });
    let submit = handle_remote_v1_request(
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&payload.to_string()),
    )
    .expect("submit request");
    assert_eq!(submit.status_code, 200);
    let submit_json: Value = serde_json::from_str(&submit.body).expect("submit json");
    assert_eq!(
        submit_json.get("execution_mode").and_then(Value::as_str),
        Some("remote_worker")
    );

    let mut result_body = None;
    for _ in 0..100 {
        let result = handle_remote_v1_request(&store, "GET", "/v1/tasks/worker-run-1/result", None)
            .expect("result request");
        if result.status_code == 200 {
            result_body = Some(result.body);
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let result_body = result_body.expect("worker should eventually publish result");
    let result_json: Value = serde_json::from_str(&result_body).expect("result json");
    assert_eq!(
        result_json.get("success").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result_json.get("status").and_then(Value::as_str),
        Some("success")
    );
    let outputs = result_json
        .get("outputs")
        .and_then(Value::as_array)
        .expect("outputs array");
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs[0].get("path").and_then(Value::as_str),
        Some("dist/out.txt")
    );

    let events = handle_remote_v1_request(
        &store,
        "GET",
        "/v1/tasks/worker-run-1/events?after_seq=0",
        None,
    )
    .expect("events request");
    assert_eq!(events.status_code, 200);
    let event_lines = events
        .body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert!(
        event_lines
            .iter()
            .any(|line| line.contains("\"type\":\"TASK_COMPLETED\"")),
        "events should contain terminal completion event"
    );

    let output_download = handle_remote_v1_request(
        &store,
        "GET",
        "/v1/tasks/worker-run-1/outputs?attempt=1&path=dist/out.txt",
        None,
    )
    .expect("output download request");
    assert_eq!(output_download.status_code, 200);
    let output_json: Value =
        serde_json::from_str(&output_download.body).expect("output download json");
    let encoded = output_json
        .get("data_base64")
        .and_then(Value::as_str)
        .expect("base64 payload");
    let downloaded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .expect("decode output bytes");
    assert_eq!(downloaded, b"hello-from-zip\n");
}
