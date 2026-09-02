use takd::SubmitAttemptStore;

use crate::support::remote_output::test_context;

const UPGRADE: &str = "upgrade tak, takd, and workers together";

#[test]
fn every_legacy_worker_route_is_only_an_upgrade_rejection() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();

    for (method, path) in [
        ("GET", "/v1/node/info"),
        ("GET", "/v1/node/ping"),
        ("GET", "/v1/node/status"),
        ("GET", "/v1/node/logs"),
        ("GET", "/v1/tasks"),
        ("POST", "/v1/tasks/submit"),
        ("GET", "/v1/tasks/legacy/events"),
        ("GET", "/v1/tasks/legacy/result"),
        ("POST", "/v1/tasks/legacy/cancel"),
        ("GET", "/v1/tasks/legacy/outputs"),
        ("POST", "/v1/workspaces/uploads"),
        ("POST", "/v1/workspaces/uploads/legacy/chunks"),
        ("POST", "/v1/workspaces/uploads/legacy/wormhole"),
    ] {
        let response = takd::daemon::remote::handle_worker_http_request(
            &test_context(),
            &store,
            method,
            path,
            &[],
            Some(b"ignored legacy payload"),
        )
        .unwrap();
        let body = String::from_utf8_lossy(&response.body).to_ascii_lowercase();

        assert_eq!(response.status_code, 426, "{method} {path}: {body}");
        assert!(body.contains(UPGRADE), "{method} {path}: {body}");
    }
}
