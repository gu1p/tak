use prost::Message;
use tak_proto::{
    CmdStep, ContainerRuntime, OutputSelector, RuntimeSpec, Step, SubmitTaskRequest, runtime_spec,
    step,
};

pub fn truncated_submit_request(task_run_id: &str) -> Vec<u8> {
    let prefix = submit_request(task_run_id, Vec::new()).encode_to_vec();
    let full = submit_request(
        task_run_id,
        vec![OutputSelector {
            kind: Some(tak_proto::output_selector::Kind::Path(
                "artifacts/stdout.txt".into(),
            )),
        }],
    )
    .encode_to_vec();
    assert!(
        full.starts_with(&prefix),
        "full protobuf must extend valid prefix"
    );
    let mut request = format!(
        concat!(
            "POST /v1/tasks/submit HTTP/1.1\r\n",
            "Host: 127.0.0.1\r\n",
            "Authorization: Bearer secret\r\n",
            "Content-Type: application/x-protobuf\r\n",
            "Content-Length: {}\r\n",
            "Connection: close\r\n",
            "\r\n"
        ),
        full.len()
    )
    .into_bytes();
    request.extend_from_slice(&prefix);
    request
}

fn submit_request(task_run_id: &str, outputs: Vec<OutputSelector>) -> SubmitTaskRequest {
    SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt: 1,
        workspace_zip: zip::ZipWriter::new(std::io::Cursor::new(Vec::new()))
            .finish()
            .expect("finish empty workspace zip")
            .into_inner(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".into(), "-c".into(), "sleep 2".into()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(test_container_runtime()),
        task_label: "//apps/web:build".into(),
        needs: Vec::new(),
        outputs,
        session: None,
    }
}

fn test_container_runtime() -> RuntimeSpec {
    RuntimeSpec {
        kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
            image: Some("alpine:3.20".into()),
            dockerfile: None,
            build_context: None,
        })),
    }
}
