use std::fs;
use std::process::Command as StdCommand;

#[path = "support/qr_output.rs"]
mod qr_output;

use qr_output::{extract_block, qr_block_body_height, visible_text};
use qrcode::QrCode;
use ratatui::layout::Rect;
use tak_proto::encode_tor_invite;
use tui_qrcode::QrCodeWidget;

#[test]
fn token_show_qr_renders_onboarding_command_and_qr_block() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let invite = encode_tor_invite("http://builder-qr.onion").expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");

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
    assert!(stdout.contains(&invite), "missing invite:\n{stdout}");
    assert!(
        stdout.contains(&format!("tak remote add '{invite}'")),
        "missing client command:\n{stdout}"
    );
    assert!(
        stdout.lines().filter(|line| line.contains('█')).count() >= 4,
        "missing QR block render:\n{stdout}"
    );

    let qr_block = extract_block(&stdout, " Takd Invite ");
    let required_qr_height = QrCodeWidget::new(QrCode::new(invite.as_bytes()).expect("qr code"))
        .size(Rect::new(0, 0, 0, 0))
        .height as usize
        + 2;
    assert!(
        qr_block_body_height(&qr_block) >= required_qr_height,
        "QR block clipped: need at least {required_qr_height} inner rows\n{stdout}"
    );

    let command = format!("tak remote add '{invite}'");
    let client_block = extract_block(&stdout, " Client ");
    assert_eq!(
        visible_text(&client_block),
        command
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>(),
        "client block should contain the full wrapped command:\n{stdout}"
    );

    let token_block = extract_block(&stdout, " Invite ");
    assert_eq!(
        visible_text(&token_block),
        invite
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>(),
        "invite block should contain the full wrapped invite:\n{stdout}"
    );
}
