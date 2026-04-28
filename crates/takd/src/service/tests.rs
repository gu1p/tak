use crate::agent::{AgentConfig, InitAgentOptions, init_agent, read_config, read_token};
use crate::test_env::{EnvGuard, env_lock};
use std::fs;
use std::time::Duration;

use super::tor::{onion_service_config, test_tor_hidden_service_bind_addr};
use super::{serve_agent, serve_direct_agent};
use crate::daemon::remote::SubmitAttemptStore;

#[tokio::test]
async fn direct_transport_requires_http_base_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let err = serve_direct_agent(
        temp.path(),
        temp.path(),
        &agent_config(None),
        submit_store(&temp),
    )
    .await
    .expect_err("missing base_url");
    assert!(err.to_string().contains("base_url must be http(s)"));
}

#[tokio::test]
async fn direct_transport_rejects_non_http_base_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let err = serve_direct_agent(
        temp.path(),
        temp.path(),
        &agent_config(Some("ssh://builder")),
        submit_store(&temp),
    )
    .await
    .expect_err("invalid scheme");
    assert!(err.to_string().contains("base_url must be http(s)"));
}

#[tokio::test]
async fn direct_transport_rejects_base_url_with_unsupported_components() {
    for base_url in [
        "http://user:pass@127.0.0.1:0",
        "http://127.0.0.1:0/prefix",
        "http://127.0.0.1:0?query=1",
        "http://127.0.0.1:0#fragment",
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_root = temp.path().join("config");
        let state_root = temp.path().join("state");
        fs::create_dir_all(&config_root).expect("create config root");
        fs::create_dir_all(&state_root).expect("create state root");
        fs::write(
            config_root.join("agent.toml"),
            toml::to_string(&agent_config(Some(base_url))).expect("encode agent config"),
        )
        .expect("write agent config");
        let err = tokio::time::timeout(
            Duration::from_millis(200),
            serve_direct_agent(
                &config_root,
                &state_root,
                &agent_config(Some(base_url)),
                submit_store(&temp),
            ),
        )
        .await
        .expect("unsupported base_url should fail before serving")
        .expect_err("unsupported base_url should fail");
        assert!(
            err.to_string()
                .contains("base_url must not include userinfo, path, query, or fragment"),
            "unexpected error for {base_url}: {err:#}"
        );
    }
}

#[test]
fn onion_service_config_validates_nicknames() {
    assert!(onion_service_config("buildertor").is_ok());
    let err = onion_service_config("bad nickname").expect_err("invalid nickname");
    assert!(format!("{err:#}").contains("invalid tor hidden-service nickname"));
}

#[test]
fn test_bind_addr_helper_trims_and_ignores_empty_values() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TEST_TOR_HS_BIND_ADDR");
    assert_eq!(test_tor_hidden_service_bind_addr(), None);
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", " 127.0.0.1:9 ");
    assert_eq!(
        test_tor_hidden_service_bind_addr().as_deref(),
        Some("127.0.0.1:9")
    );
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", "   ");
    assert_eq!(test_tor_hidden_service_bind_addr(), None);
}

#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn tor_test_bind_override_keeps_token_pending_until_listener_binds() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let occupied_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind occupied");
    let bind_addr = occupied_listener
        .local_addr()
        .expect("occupied addr")
        .to_string();
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", &bind_addr);

    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    let empty = Vec::<String>::new();
    init_agent(
        &config_root,
        &state_root,
        InitAgentOptions {
            node_id: Some("builder-tor"),
            display_name: None,
            transport: Some("tor"),
            base_url: None,
            pools: &empty,
            tags: &empty,
            capabilities: &empty,
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init tor agent");

    let err = serve_agent(&config_root, &state_root)
        .await
        .expect_err("bind should fail while address is occupied");
    assert!(
        format!("{err:#}").contains("bind takd tor test listener"),
        "unexpected error: {err:#}"
    );
    assert!(
        read_token(&state_root).is_err(),
        "token should stay pending"
    );
    assert_eq!(
        read_config(&config_root).expect("read config").base_url,
        None,
        "base_url should stay pending after bind failure"
    );
}

fn agent_config(base_url: Option<&str>) -> AgentConfig {
    AgentConfig {
        node_id: "builder".into(),
        display_name: "builder".into(),
        base_url: base_url.map(str::to_string),
        bearer_token: "secret".into(),
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
        hidden_service_nickname: "builder".into(),
        image_cache: None,
    }
}

fn submit_store(temp: &tempfile::TempDir) -> SubmitAttemptStore {
    SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("submit store")
}
