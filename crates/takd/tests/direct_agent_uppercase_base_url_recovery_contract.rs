mod support;

use std::fs;
use std::net::TcpListener;
use std::process::{Command as StdCommand, Stdio};

use support::cli::{roots, takd_bin};
use tak_proto::decode_remote_token;
use takd::agent::AgentConfig;

#[test]
fn direct_serve_normalizes_existing_uppercase_fixed_port_base_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = roots(temp.path());
    fs::create_dir_all(&config_root).expect("create config root");
    fs::create_dir_all(&state_root).expect("create state root");

    let port = reserved_port();
    let configured_base_url = format!("HTTP://127.0.0.1:{port}");
    let canonical_base_url = format!("http://127.0.0.1:{port}");
    fs::write(
        config_root.join("agent.toml"),
        toml::to_string(&agent_config(&configured_base_url)).expect("encode agent config"),
    )
    .expect("write agent config");

    let mut child = StdCommand::new(takd_bin())
        .args([
            "serve",
            "--config-root",
            &config_root.display().to_string(),
            "--state-root",
            &state_root.display().to_string(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn takd serve");

    let show = StdCommand::new(takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "5",
        ])
        .output()
        .expect("run token show");
    child.kill().expect("kill takd serve");
    child.wait().expect("wait takd serve");

    assert!(
        show.status.success(),
        "token show should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&show.stdout),
        String::from_utf8_lossy(&show.stderr)
    );
    let config = fs::read_to_string(config_root.join("agent.toml")).expect("read config");
    assert!(
        config.contains(&format!("base_url = \"{canonical_base_url}\"")),
        "expected serve to rewrite base_url to lowercase:\n{config}"
    );
    assert!(
        !config.contains(&format!("base_url = \"{configured_base_url}\"")),
        "did not expect mixed-case base_url to persist:\n{config}"
    );

    let token = String::from_utf8(show.stdout).expect("token stdout utf8");
    let payload = decode_remote_token(token.trim()).expect("decode token");
    assert_eq!(
        payload.node.expect("node info in token").base_url,
        canonical_base_url
    );
}

fn agent_config(base_url: &str) -> AgentConfig {
    AgentConfig {
        node_id: "builder".into(),
        display_name: "builder".into(),
        base_url: Some(base_url.into()),
        bearer_token: "secret".into(),
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
        hidden_service_nickname: "builder".into(),
    }
}

fn reserved_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("listener addr").port()
}
