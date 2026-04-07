use std::fs;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};

#[test]
fn token_show_waits_for_hidden_service_token() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let token_path = state_root.join("agent.token");
    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".into(),
        node: Some(NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
        }),
        bearer_token: "secret".into(),
    })
    .expect("encode token");

    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        fs::write(token_path, format!("{token}\n")).expect("write token");
    });

    let show = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "2",
        ])
        .output()
        .expect("run takd token show --wait");
    writer.join().expect("writer should exit");

    assert!(
        show.status.success(),
        "takd token show --wait should succeed"
    );
    assert!(
        String::from_utf8_lossy(&show.stdout)
            .trim()
            .starts_with("takd:v1:"),
        "unexpected stdout:\n{}",
        String::from_utf8_lossy(&show.stdout)
    );
}
