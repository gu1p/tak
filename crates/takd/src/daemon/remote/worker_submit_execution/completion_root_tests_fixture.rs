use tak_core::model::StepDef;

use super::*;

pub(super) fn blocking_payload(run: &str) -> RemoteWorkerSubmitPayload {
    RemoteWorkerSubmitPayload {
        workspace_zip: zip::ZipWriter::new(std::io::Cursor::new(Vec::new()))
            .finish()
            .expect("finish empty workspace zip")
            .into_inner(),
        task_run_id: run.into(),
        task_label: "//:check".into(),
        attempt: 1,
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".into(),
                "-c".into(),
                "touch ready; while [ ! -e release ]; do sleep 0.01; done".into(),
            ],
            cwd: None,
            env: Default::default(),
        }],
        timeout_s: Some(45),
        runtime: None,
        needs: Vec::new(),
        outputs: Vec::new(),
        session: Some(RemoteWorkerSession {
            key: "shared".into(),
            reuse: RemoteWorkerSessionReuse::ShareWorkspace,
        }),
        fused_members: Vec::new(),
        origin: Some("task".into()),
        runtime_source: None,
        command: Some("blocking local fixture".into()),
        execution_label: None,
    }
}
