use std::path::Path;
use tak_core::model::Scope;
use takd::daemon::{
    lease::{new_shared_manager, new_shared_manager_with_db},
    protocol::{Request, Response, RunTasksRequest, StatusRequest, run_server},
    remote::{
        SubmitAttemptStore, build_submit_idempotency_key, handle_remote_v1_request,
        run_remote_v1_http_server,
    },
    runtime::{default_socket_path, default_state_db_path, run_daemon},
    transport::{
        ArtiSettings, ContainerEngine, ContainerEngineProbe, HostPlatform, TorTransportConfig,
        normalize_tor_transport_config, select_container_engine,
        select_container_engine_with_probe, validate_tor_transport_config,
    },
};

struct ProbeOkDocker;

impl ContainerEngineProbe for ProbeOkDocker {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
        match engine {
            ContainerEngine::Docker => Ok(()),
            ContainerEngine::Podman => Err("podman unavailable".to_string()),
        }
    }
}

#[test]
fn namespaced_daemon_api_surface_is_stable() {
    let _run_daemon_fn = run_daemon;
    let _run_server_fn = run_server;
    let _run_http_fn = run_remote_v1_http_server;

    let _socket = default_socket_path();
    let _db = default_state_db_path();

    let mut probe = ProbeOkDocker;
    let selected = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)
        .expect("select engine with probe");
    assert_eq!(selected, ContainerEngine::Docker);
    let selected_current = select_container_engine(&mut probe).expect("select engine");
    assert_eq!(selected_current, ContainerEngine::Docker);

    let config = TorTransportConfig {
        onion_endpoint:
            "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion"
                .to_string(),
        service_auth_token: "service-token-123".to_string(),
        arti: ArtiSettings {
            socks5_addr: "127.0.0.1:9150".to_string(),
            data_dir: "/tmp/tak/arti".to_string(),
        },
    };
    validate_tor_transport_config(&config).expect("valid tor config");
    let normalized = normalize_tor_transport_config(config).expect("normalized tor config");
    assert_eq!(normalized.arti.socks5_addr, "127.0.0.1:9150");

    let manager = new_shared_manager();
    let status_request = Request::Status(StatusRequest {
        request_id: "req-status".to_string(),
    });
    if let Request::Status(payload) = status_request {
        assert_eq!(payload.request_id, "req-status");
    } else {
        panic!("status request variant");
    }

    let _run_request = Request::RunTasks(RunTasksRequest {
        request_id: "req-run".to_string(),
        workspace_root: ".".to_string(),
        labels: vec!["//:task".to_string()],
        jobs: 1,
        keep_going: false,
        lease_socket: None,
        lease_ttl_ms: 5_000,
        lease_poll_interval_ms: 250,
        session_id: Some("session-1".to_string()),
        user: Some("user-1".to_string()),
    });
    let _response = Response::StatusSnapshot {
        request_id: "req-status".to_string(),
        status: manager.lock().expect("lock manager").status(),
    };

    let temp = tempfile::tempdir().expect("tempdir");
    let store_path = temp.path().join("takd.sqlite");
    let _manager_with_db = new_shared_manager_with_db(store_path.clone()).expect("manager with db");
    let store = SubmitAttemptStore::with_db_path(store_path).expect("submit store");

    let key = build_submit_idempotency_key("task-run-1", Some(1)).expect("idempotency key");
    assert_eq!(key, "task-run-1:1");

    let response =
        handle_remote_v1_request(&store, "GET", "/v1/node/status", None).expect("status response");
    assert_eq!(response.status_code, 200);

    assert_eq!(Scope::Machine, Scope::Machine);
    assert!(Path::new("/tmp").is_absolute());
}
