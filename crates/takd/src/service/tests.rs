use crate::agent::AgentConfig;
use crate::test_env::{EnvGuard, env_lock};

use super::*;

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
    }
}

fn submit_store(temp: &tempfile::TempDir) -> SubmitAttemptStore {
    SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("submit store")
}
