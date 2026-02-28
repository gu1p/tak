use super::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Duration;
use tak_core::model::BackoffDef;
use tak_core::model::CurrentStateSpec;
use tak_core::model::Hold;
use tak_core::model::LimiterRef;
use tak_core::model::NeedDef;
use tak_core::model::ResolvedTask;
use tak_core::model::RetryDef;
use tak_core::model::Scope;
use tak_core::model::TaskExecutionSpec;
use tak_core::model::TaskLabel;
use tak_core::model::WorkspaceSpec;

fn strict_remote_target(kind: RemoteTransportKind, endpoint: &str) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "node-a".to_string(),
        endpoint: endpoint.to_string(),
        transport_kind: kind,
        service_auth_env: None,
        runtime: None,
    }
}

#[test]
fn transport_factory_selects_direct_transport_variant() {
    assert_eq!(
        TransportFactory::transport_name(RemoteTransportKind::DirectHttps),
        "direct"
    );
}

#[test]
fn transport_factory_selects_tor_transport_variant() {
    assert_eq!(
        TransportFactory::transport_name(RemoteTransportKind::Tor),
        "tor"
    );
}

#[test]
fn transport_factory_resolves_socket_addr_for_supported_transports() {
    for kind in [RemoteTransportKind::DirectHttps, RemoteTransportKind::Tor] {
        let target = strict_remote_target(kind, "http://127.0.0.1:4242");
        let socket_addr = TransportFactory::socket_addr(&target)
            .expect("socket address should resolve for supported transport");
        assert_eq!(socket_addr, "127.0.0.1:4242");
    }
}

#[test]
fn endpoint_socket_addr_defaults_port_by_scheme_when_missing() {
    let https = strict_remote_target(RemoteTransportKind::DirectHttps, "https://build.internal");
    let tor_http = strict_remote_target(
        RemoteTransportKind::Tor,
        "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion",
    );

    assert_eq!(
        TransportFactory::socket_addr(&https).expect("https without explicit port"),
        "build.internal:443"
    );
    assert_eq!(
        TransportFactory::socket_addr(&tor_http).expect("onion http without explicit port"),
        "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion:80"
    );
}

#[test]
fn endpoint_socket_addr_accepts_full_url_forms_without_explicit_port() {
    let direct_full_url = strict_remote_target(
        RemoteTransportKind::DirectHttps,
        "https://build.internal?region=us-east#ignored",
    );
    let tor_full_url = strict_remote_target(
        RemoteTransportKind::Tor,
        "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion?queue=default#anchor",
    );

    assert_eq!(
        TransportFactory::socket_addr(&direct_full_url).expect("direct full URL"),
        "build.internal:443"
    );
    assert_eq!(
        TransportFactory::socket_addr(&tor_full_url).expect("tor full URL"),
        "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion:80"
    );
}

#[test]
fn transport_variant_branching_isolated_to_transport_factory() {
    let source = include_str!("lib.rs");
    let production = source.split("\n#[cfg(test)]").next().unwrap_or(source);
    let sites = production
        .lines()
        .filter(|line| line.contains("RemoteTransportKind::"))
        .map(|line| line.trim().to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        sites,
        vec![
            "RemoteTransportKind::DirectHttps => &DIRECT_HTTPS_TRANSPORT_ADAPTER,".to_string(),
            "RemoteTransportKind::Tor => &TOR_TRANSPORT_ADAPTER,".to_string(),
        ],
        "transport variant branching must remain isolated to TransportFactory::adapter"
    );
}

#[test]
fn ndjson_single_line_event_is_not_treated_as_wrapped_done_payload() {
    let target = strict_remote_target(RemoteTransportKind::DirectHttps, "http://127.0.0.1:4242");
    let response_body =
        r#"{"seq":1,"type":"TASK_LOG_CHUNK","payload":{"kind":"TASK_LOG_CHUNK","chunk":"hello"}}"#;

    let parsed =
        parse_remote_events_response(&target, response_body, 0).expect("single NDJSON line parse");
    assert_eq!(parsed.next_seq, 1);
    assert!(!parsed.done);
    assert_eq!(
        parsed.remote_logs,
        vec![RemoteLogChunk {
            seq: 1,
            chunk: "hello".to_string(),
        }]
    );
}

#[test]
fn remote_events_max_wait_defaults_to_120_seconds() {
    assert_eq!(
        parse_remote_events_max_wait_duration(None),
        Duration::from_secs(120)
    );
    assert_eq!(
        parse_remote_events_max_wait_duration(Some("")),
        Duration::from_secs(120)
    );
    assert_eq!(
        parse_remote_events_max_wait_duration(Some("0")),
        Duration::from_secs(120)
    );
    assert_eq!(
        parse_remote_events_max_wait_duration(Some("invalid")),
        Duration::from_secs(120)
    );
}

#[test]
fn remote_events_max_wait_accepts_positive_seconds_override() {
    assert_eq!(
        parse_remote_events_max_wait_duration(Some("900")),
        Duration::from_secs(900)
    );
}

#[test]
fn should_retry_matches_exit_filter_rules() {
    assert!(should_retry(Some(7), &[]));
    assert!(should_retry(None, &[]));

    assert!(should_retry(Some(42), &[42, 43]));
    assert!(!should_retry(Some(9), &[42, 43]));
    assert!(!should_retry(None, &[42, 43]));
}

#[test]
fn retry_backoff_delay_respects_fixed_and_exp_jitter_bounds() {
    let fixed = BackoffDef::Fixed { seconds: 1.5 };
    assert_eq!(retry_backoff_delay(&fixed, 1), Duration::from_millis(1500));

    let exp = BackoffDef::ExpJitter {
        min_s: 0.5,
        max_s: 5.0,
        jitter: "full".to_string(),
    };
    assert_eq!(retry_backoff_delay(&exp, 1), Duration::from_millis(500));
    assert_eq!(retry_backoff_delay(&exp, 2), Duration::from_secs(1));
    assert_eq!(retry_backoff_delay(&exp, 5), Duration::from_secs(5));
    assert_eq!(retry_backoff_delay(&exp, 20), Duration::from_secs(5));
}

#[test]
fn seconds_to_duration_clamps_invalid_values() {
    assert_eq!(retry::seconds_to_duration(-10.0), Duration::ZERO);
    assert_eq!(retry::seconds_to_duration(0.0), Duration::ZERO);
    assert_eq!(retry::seconds_to_duration(f64::NAN), Duration::ZERO);
    assert_eq!(retry::seconds_to_duration(f64::INFINITY), Duration::ZERO);
    assert_eq!(
        retry::seconds_to_duration(2.25),
        Duration::from_millis(2250),
        "positive finite values convert directly"
    );
}

#[test]
fn target_set_from_summary_returns_all_result_labels() {
    let mut results = BTreeMap::new();
    let a = TaskLabel {
        package: "//pkg".to_string(),
        name: "a".to_string(),
    };
    let b = TaskLabel {
        package: "//pkg".to_string(),
        name: "b".to_string(),
    };
    results.insert(
        a.clone(),
        TaskRunResult {
            attempts: 1,
            success: true,
            exit_code: Some(0),
            placement_mode: PlacementMode::Local,
            remote_node_id: None,
            remote_transport_kind: None,
            decision_reason: None,
            context_manifest_hash: None,
            remote_runtime_kind: None,
            remote_runtime_engine: None,
            remote_logs: Vec::new(),
            synced_outputs: Vec::new(),
        },
    );
    results.insert(
        b.clone(),
        TaskRunResult {
            attempts: 2,
            success: false,
            exit_code: Some(7),
            placement_mode: PlacementMode::Remote,
            remote_node_id: Some("n1".to_string()),
            remote_transport_kind: Some("tor".to_string()),
            decision_reason: Some("test".to_string()),
            context_manifest_hash: Some("abc".to_string()),
            remote_runtime_kind: Some("containerized".to_string()),
            remote_runtime_engine: Some("docker".to_string()),
            remote_logs: Vec::new(),
            synced_outputs: Vec::new(),
        },
    );

    let summary = RunSummary { results };
    let got = target_set_from_summary(&summary);
    let expected = HashSet::from([a, b]);
    assert_eq!(got, expected);
}

fn test_label(package: &str, name: &str) -> TaskLabel {
    TaskLabel {
        package: package.to_string(),
        name: name.to_string(),
    }
}

fn test_task(label: TaskLabel, deps: Vec<TaskLabel>) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps,
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::default(),
        tags: Vec::new(),
    }
}

fn test_workspace(tasks: Vec<ResolvedTask>) -> WorkspaceSpec {
    let tasks = tasks
        .into_iter()
        .map(|task| (task.label.clone(), task))
        .collect::<BTreeMap<_, _>>();
    WorkspaceSpec {
        project_id: "p".to_string(),
        root: std::env::temp_dir(),
        tasks,
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

#[test]
fn collect_required_labels_returns_transitive_closure() {
    let a = test_label("//app", "a");
    let b = test_label("//app", "b");
    let c = test_label("//app", "c");
    let d = test_label("//app", "d");
    let workspace = test_workspace(vec![
        test_task(a.clone(), vec![b.clone()]),
        test_task(b.clone(), vec![c.clone()]),
        test_task(c.clone(), Vec::new()),
        test_task(d.clone(), Vec::new()),
    ]);

    let got = collect_required_labels(&workspace, std::slice::from_ref(&a))
        .expect("closure should resolve");
    let expected = BTreeSet::from([a, b, c]);
    assert_eq!(got, expected);
}

#[test]
fn collect_required_labels_rejects_missing_target() {
    let known = test_label("//app", "known");
    let missing = test_label("//app", "missing");
    let workspace = test_workspace(vec![test_task(known, Vec::new())]);

    let err = collect_required_labels(&workspace, &[missing]).expect_err("target should fail");
    assert!(
        err.to_string().contains("does not exist"),
        "unexpected error: {err}"
    );
}

#[test]
fn resolve_cwd_returns_workspace_root_when_unset() {
    let root = std::env::temp_dir().join("tak-exec-resolve-cwd-default");
    let got = resolve_cwd(&root, &None);
    assert_eq!(got, root);
}

#[test]
fn resolve_cwd_resolves_relative_paths_under_workspace_root() {
    let root = std::env::temp_dir().join("tak-exec-resolve-cwd-relative");
    let got = resolve_cwd(&root, &Some("apps/web".to_string()));
    assert_eq!(got, root.join("apps/web"));
}

#[test]
fn resolve_cwd_preserves_absolute_paths() {
    let root = std::env::temp_dir().join("tak-exec-resolve-cwd-absolute-root");
    let absolute = std::env::temp_dir().join("tak-exec-absolute-cwd");
    let got = resolve_cwd(&root, &Some(absolute.display().to_string()));
    assert_eq!(got, absolute);
}

#[test]
fn convert_needs_maps_limiter_fields_into_wire_shape() {
    let needs = vec![
        NeedDef {
            limiter: LimiterRef {
                name: "cpu".to_string(),
                scope: Scope::Machine,
                scope_key: None,
            },
            slots: 2.5,
            hold: Hold::During,
        },
        NeedDef {
            limiter: LimiterRef {
                name: "queue".to_string(),
                scope: Scope::Project,
                scope_key: Some("p:demo".to_string()),
            },
            slots: 1.0,
            hold: Hold::AtStart,
        },
    ];

    let mapped = lease_client::convert_needs(&needs);
    assert_eq!(mapped.len(), 2);
    assert_eq!(mapped[0].name, "cpu");
    assert_eq!(mapped[0].scope, Scope::Machine);
    assert_eq!(mapped[0].scope_key, None);
    assert_eq!(mapped[0].slots, 2.5);
    assert_eq!(mapped[1].name, "queue");
    assert_eq!(mapped[1].scope, Scope::Project);
    assert_eq!(mapped[1].scope_key.as_deref(), Some("p:demo"));
    assert_eq!(mapped[1].slots, 1.0);
}

#[tokio::test]
async fn acquire_task_lease_without_socket_returns_none() {
    let task = test_task(test_label("//app", "needs"), Vec::new());
    let options = RunOptions {
        lease_socket: None,
        ..RunOptions::default()
    };
    let context = LeaseContext {
        user: "u".to_string(),
        session_id: "s".to_string(),
    };

    let lease = acquire_task_lease(&task, 1, &options, &context)
        .await
        .expect("no socket should short-circuit");
    assert_eq!(lease, None);
}
