use tak_proto::{CmdStep, Step, SubmitTaskRequest, WorkspaceUploadRef, step};

pub(crate) fn submit_with_upload(
    upload_id: &str,
    sha256: &str,
    size_bytes: u64,
) -> SubmitTaskRequest {
    SubmitTaskRequest {
        task_run_id: "run-escape".into(),
        attempt: 1,
        workspace_zip: Vec::new(),
        steps: vec![Step {
            kind: Some(step::Kind::Cmd(CmdStep {
                argv: vec!["sh".into(), "-c".into(), "true".into()],
                cwd: None,
                env: Default::default(),
            })),
        }],
        timeout_s: None,
        runtime: Some(crate::support::remote_output::test_container_runtime()),
        task_label: "//apps/web:test".into(),
        needs: Vec::new(),
        outputs: Vec::new(),
        session: None,
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("sh -c 'true'".into()),
        fused_members: Vec::new(),
        execution_label: None,
        workspace_upload: Some(WorkspaceUploadRef {
            upload_id: upload_id.into(),
            sha256: sha256.into(),
            size_bytes,
        }),
    }
}
