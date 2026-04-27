use super::*;

#[test]
fn numeric_container_user_defaults_home_to_workspace() {
    let mut step_spec = ContainerStepSpec {
        argv: vec!["true".to_string()],
        cwd: PathBuf::from("/workspace"),
        env: BTreeMap::new(),
    };

    apply_container_user_defaults(
        &mut step_spec,
        Path::new("/var/tmp/takd-remote-exec/job-1"),
        Some("1000:1000"),
    );

    assert_eq!(
        step_spec.env.get("HOME").map(String::as_str),
        Some("/var/tmp/takd-remote-exec/job-1")
    );
}

#[test]
fn numeric_container_user_preserves_explicit_home() {
    let mut step_spec = ContainerStepSpec {
        argv: vec!["true".to_string()],
        cwd: PathBuf::from("/workspace"),
        env: BTreeMap::from([("HOME".to_string(), "/custom".to_string())]),
    };

    apply_container_user_defaults(
        &mut step_spec,
        Path::new("/var/tmp/takd-remote-exec/job-1"),
        Some("1000:1000"),
    );

    assert_eq!(
        step_spec.env.get("HOME").map(String::as_str),
        Some("/custom")
    );
}

#[test]
fn named_container_user_does_not_default_home() {
    let mut step_spec = ContainerStepSpec {
        argv: vec!["true".to_string()],
        cwd: PathBuf::from("/workspace"),
        env: BTreeMap::new(),
    };

    apply_container_user_defaults(
        &mut step_spec,
        Path::new("/var/tmp/takd-remote-exec/job-1"),
        Some("root"),
    );

    assert_eq!(step_spec.env.get("HOME"), None);
}
