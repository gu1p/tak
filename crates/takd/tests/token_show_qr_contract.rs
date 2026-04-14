use std::fs;
use std::process::Command as StdCommand;

#[path = "support/qr_output.rs"]
mod qr_output;

use qr_output::{extract_block, qr_block_body_height, visible_text};
use qrcode::QrCode;
use ratatui::layout::Rect;
use tak_proto::{NodeInfo, RemoteTokenPayload, encode_remote_token};
use tui_qrcode::QrCodeWidget;

#[test]
fn token_show_qr_renders_onboarding_command_and_qr_block() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let token = encode_remote_token(&RemoteTokenPayload {
        version: "v1".into(),
        node: Some(NodeInfo {
            node_id: "builder-qr".into(),
            display_name: "builder-qr".into(),
            base_url: "http://builder-qr.onion".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        bearer_token: "secret".into(),
    })
    .expect("encode token");
    fs::write(state_root.join("agent.token"), format!("{token}\n")).expect("write token");

    let show = StdCommand::new(assert_cmd::cargo::cargo_bin!("takd"))
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--qr",
        ])
        .output()
        .expect("run takd token show --qr");

    assert!(show.status.success(), "takd token show --qr should succeed");
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(
        stdout.contains("Scan this QR code"),
        "missing QR label:\n{stdout}"
    );
    assert!(stdout.contains(&token), "missing token:\n{stdout}");
    assert!(
        stdout.contains(&format!("tak remote add '{token}'")),
        "missing client command:\n{stdout}"
    );
    assert!(
        stdout.lines().filter(|line| line.contains('█')).count() >= 4,
        "missing QR block render:\n{stdout}"
    );

    let qr_block = extract_block(&stdout, " Takd Token ");
    let required_qr_height = QrCodeWidget::new(QrCode::new(token.as_bytes()).expect("qr code"))
        .size(Rect::new(0, 0, 0, 0))
        .height as usize
        + 2;
    assert!(
        qr_block_body_height(&qr_block) >= required_qr_height,
        "QR block clipped: need at least {required_qr_height} inner rows\n{stdout}"
    );

    let command = format!("tak remote add '{token}'");
    let client_block = extract_block(&stdout, " Client ");
    assert_eq!(
        visible_text(&client_block),
        command
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>(),
        "client block should contain the full wrapped command:\n{stdout}"
    );

    let token_block = extract_block(&stdout, " Token ");
    assert_eq!(
        visible_text(&token_block),
        token
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>(),
        "token block should contain the full wrapped token:\n{stdout}"
    );
}
