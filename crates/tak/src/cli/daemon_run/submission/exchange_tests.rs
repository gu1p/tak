use std::time::Duration;

use tak_proto::local_daemon::v2::Operation;

use super::exchange;

#[test]
fn workspace_uploads_allow_slow_durable_persistence() {
    let upload = Operation::UploadWorkspace {
        run_id: "run-slow-upload".into(),
        workspace_fingerprint: "workspace".into(),
        archive_size: 1,
        offset: 0,
        chunk: vec![0],
    };

    assert_eq!(exchange::request_timeout(&upload), Duration::from_secs(300));
    assert_eq!(
        exchange::request_timeout(&Operation::CommitRun {
            run_id: "run-slow-upload".into(),
        }),
        Duration::from_secs(30)
    );
}
