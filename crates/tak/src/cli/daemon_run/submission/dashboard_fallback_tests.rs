use anyhow::anyhow;

use crate::cli::run_dashboard::fallback::safe_warning;

#[test]
fn fallback_warning_cannot_inject_terminal_controls() {
    let warning = safe_warning(
        "during workspace\nupload",
        &anyhow!("draw failed\x1b[2J\rforged"),
    );

    assert!(warning.contains("daemon-owned run continues"), "{warning}");
    assert!(
        warning.chars().all(|character| !character.is_control()),
        "{warning:?}"
    );
}
