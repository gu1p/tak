use std::io::Write;
use std::thread;
use std::time::Duration;

use prost::Message;
use tak_proto::{CmdStep, NodeInfo, NodeStatusResponse, Step, SubmitTaskRequest, step};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, handle_remote_v1_request};

#[test]
fn remote_status_route_serves_protobuf_and_reports_running_job() {
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");

    let submit = SubmitTaskRequest {
        task_run_id: "task-run-1".to_string(),
        attempt: 1,
        workspace_zip: empty_workspace_zip(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".to_string(), "-c".to_string(), "sleep 1".to_string()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: None,
        task_label: "//apps/web:build".to_string(),
        needs: vec![tak_proto::SubmittedNeed {
            name: "cpu".to_string(),
            scope: "machine".to_string(),
            scope_key: None,
            slots: 2.0,
        }],
        outputs: Vec::new(),
    };
    let submit = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");
    assert_eq!(submit.status_code, 200);

    for _ in 0..50 {
        let response = handle_remote_v1_request(&context, &store, "GET", "/v1/node/status", None)
            .expect("status response");
        assert_eq!(response.content_type, "application/x-protobuf");
        let status =
            NodeStatusResponse::decode(response.body.as_slice()).expect("decode node status");
        if !status.active_jobs.is_empty() {
            assert_eq!(status.node.expect("node").node_id, "builder-a");
            assert_eq!(status.active_jobs[0].task_label, "//apps/web:build");
            assert_eq!(status.active_jobs[0].attempt, 1);
            assert_eq!(status.active_jobs[0].needs.len(), 1);
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }

    panic!("timed out waiting for active job in node status");
}

fn empty_workspace_zip() -> Vec<u8> {
    let cursor = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(cursor);
    writer
        .start_file("TASKS.py", zip::write::SimpleFileOptions::default())
        .expect("start workspace file");
    writer
        .write_all(b"SPEC = module_spec(tasks=[])\nSPEC\n")
        .expect("write workspace file");
    writer.finish().expect("finish workspace zip").into_inner()
}
