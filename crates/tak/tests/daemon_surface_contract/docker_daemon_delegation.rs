use std::collections::BTreeMap;

use crate::support::run_tak_output;

#[test]
fn docker_run_requires_takd_even_for_local_placement() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "missing-takd.sock".into())]);

    let output = run_tak_output(
        temp.path(),
        &["--local", "docker", "run", "alpine:3.20", "true"],
        &environment,
    )
    .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("Local takd is unavailable"), "{stderr}");
    assert!(stderr.contains("no client execution fallback"), "{stderr}");
}
