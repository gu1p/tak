use std::collections::BTreeMap;

use crate::support::run_tak_output;

#[test]
fn make_reports_every_missing_repeatable_pass_env_before_submission() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    std::fs::write(temp.path().join("Makefile"), "check:\n\t@true\n").unwrap();
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "missing.sock".into())]);

    let output = run_tak_output(
        temp.path(),
        &[
            "make",
            "check",
            "--pass-env",
            "TOKEN_A",
            "--pass-env",
            "TOKEN_B",
        ],
        &environment,
    )
    .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("TOKEN_A") && stderr.contains("TOKEN_B"),
        "{stderr}"
    );
    assert!(stderr.contains("missing requested environment"), "{stderr}");
}
