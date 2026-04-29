use std::path::PathBuf;

#[test]
fn linux_installer_waits_long_enough_for_live_tor_onboarding() {
    let installer =
        std::fs::read_to_string(repo_root().join("get-takd.sh")).expect("read get-takd installer");

    assert!(
        installer.contains("TAKD_WAIT_TIMEOUT_SECS=\"${TAKD_WAIT_TIMEOUT_SECS:-360}\""),
        "installer token wait must cover the live Tor startup window:\n{installer}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}
