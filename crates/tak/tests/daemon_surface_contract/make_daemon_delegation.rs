use std::collections::BTreeMap;

use crate::support::run_tak_output;

#[test]
fn make_requires_takd_and_never_executes_the_goal_in_the_client() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    std::fs::write(
        temp.path().join("Makefile"),
        "check:\n\t@printf client > client-executed\n",
    )
    .unwrap();
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "missing-takd.sock".into())]);

    let output = run_tak_output(temp.path(), &["make", "check"], &environment).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("local takd"), "{stderr}");
    assert!(!temp.path().join("client-executed").exists());
}
