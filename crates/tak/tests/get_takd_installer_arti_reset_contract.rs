use crate::support;

use std::fs;
use std::path::{Path, PathBuf};

use support::installer::{fake_systemctl, rerun_installer, run_installer};

#[test]
fn linux_installer_rerun_resets_rebuildable_arti_state_only() {
    let (temp, home, output) =
        run_installer(fake_systemctl(), &[("TAKD_DISPLAY_NAME", "identity-agent")]);
    assert!(output.status.success(), "initial installer should succeed");

    let state_root = home.join(".local/state/takd");
    let config_path = home.join(".config/takd/agent.toml");
    let original_config = fs::read_to_string(&config_path).expect("read agent config");
    write_fixture_tree(&state_root);

    let rerun = rerun_installer(&temp, &home, &[]);
    assert!(
        rerun.status.success(),
        "rerun installer should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rerun.stdout),
        String::from_utf8_lossy(&rerun.stderr)
    );

    assert_removed(&state_root.join("arti/cache"));
    assert_removed(&state_root.join("arti/state/state"));
    assert_removed(&state_root.join("arti/state/hss"));
    assert_removed(&state_root.join("arti/state/hss_iptreplay"));
    assert_removed(&state_root.join("arti/state/pt_state"));
    assert_eq!(fs::read_to_string(config_path).unwrap(), original_config);
    assert_eq!(
        fs::read_to_string(state_root.join("token.toml")).unwrap(),
        "token"
    );
    assert_eq!(
        fs::read_to_string(state_root.join("transport-health.toml")).unwrap(),
        "health"
    );
    assert_eq!(
        fs::read_to_string(state_root.join("arti/state/keystore/onion-key")).unwrap(),
        "identity"
    );
}

#[test]
fn installer_does_not_parse_arti_guard_failure_logs() {
    let installer = fs::read_to_string(repo_root().join("get-takd.sh")).expect("read installer");

    for pattern in ["Rejected 60/60", "AllGuardsDown", "descriptor"] {
        assert!(
            !installer.contains(pattern),
            "installer should reset Arti state without parsing `{pattern}`:\n{installer}"
        );
    }
}

fn write_fixture_tree(state_root: &Path) {
    write_file(state_root.join("token.toml"), "token");
    write_file(state_root.join("transport-health.toml"), "health");
    write_file(state_root.join("arti/cache/microdescs"), "cache");
    write_file(
        state_root.join("arti/state/state/guards.json"),
        "stale guards",
    );
    write_file(state_root.join("arti/state/hss/descriptor"), "runtime");
    write_file(
        state_root.join("arti/state/hss_iptreplay/replay"),
        "runtime",
    );
    write_file(state_root.join("arti/state/pt_state/state"), "runtime");
    write_file(state_root.join("arti/state/keystore/onion-key"), "identity");
}

fn write_file(path: PathBuf, contents: &str) {
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    fs::write(path, contents).expect("write fixture");
}

fn assert_removed(path: &Path) {
    assert!(
        !path.exists(),
        "installer rerun should remove rebuildable Arti path {}",
        path.display()
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
