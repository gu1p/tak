use std::process::Command;

#[test]
fn local_attempt_wrapper_command_is_hidden_but_has_an_explicit_request_contract() {
    let top = Command::new(crate::support::takd_bin())
        .arg("--help")
        .output()
        .unwrap();
    assert!(top.status.success());
    assert!(!String::from_utf8_lossy(&top.stdout).contains("__local-attempt"));

    let hidden = Command::new(crate::support::takd_bin())
        .args(["__local-attempt", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&hidden.stdout);
    assert!(hidden.status.success(), "{}", String::from_utf8_lossy(&hidden.stderr));
    assert!(stdout.contains("--request <REQUEST>"), "{stdout}");
}
