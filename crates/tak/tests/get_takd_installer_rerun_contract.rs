use crate::support;

use std::fs;

use support::installer::{fake_systemctl, rerun_installer, run_installer};

#[test]
fn linux_installer_rerun_preserves_existing_agent_config() {
    let (temp, home, output) =
        run_installer(fake_systemctl(), &[("TAKD_DISPLAY_NAME", "first-agent")]);
    assert!(
        output.status.success(),
        "initial installer should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let config_path = home.join(".config/takd/agent.toml");
    let original_config = fs::read_to_string(&config_path).expect("read initial config");
    assert!(original_config.contains("first-agent"));

    let rerun = rerun_installer(&temp, &home, &[("TAKD_DISPLAY_NAME", "second-agent")]);

    assert!(
        rerun.status.success(),
        "rerun installer should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&rerun.stdout),
        String::from_utf8_lossy(&rerun.stderr)
    );
    let rerun_config = fs::read_to_string(config_path).expect("read rerun config");
    assert_eq!(rerun_config, original_config);
    assert!(!rerun_config.contains("second-agent"));
}
