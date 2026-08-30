use crate::v2_resolved_run_support::sample_run;

#[test]
fn step_paths_cannot_escape_the_private_workspace() {
    for step in [
        tak_core::v2::Step::Cmd {
            argv: vec!["true".into()],
            cwd: Some("/tmp".into()),
            env: Default::default(),
        },
        tak_core::v2::Step::Cmd {
            argv: vec!["true".into()],
            cwd: Some("nested/../../outside".into()),
            env: Default::default(),
        },
        tak_core::v2::Step::Script {
            path: "../outside.sh".into(),
            argv: vec![],
            interpreter: None,
            cwd: Some(".".into()),
            env: Default::default(),
        },
        tak_core::v2::Step::Script {
            path: "./script.sh".into(),
            argv: vec![],
            interpreter: None,
            cwd: Some("../outside".into()),
            env: Default::default(),
        },
    ] {
        let mut run = sample_run();
        run.tasks[0].steps = vec![step];
        assert!(run.validate().is_err());
    }

    for (path, cwd) in [("./script.sh", "./subdir"), ("a/../script.sh", "a/../b")] {
        let mut run = sample_run();
        run.tasks[0].steps = vec![tak_core::v2::Step::Script {
            path: path.into(),
            argv: vec![],
            interpreter: None,
            cwd: Some(cwd.into()),
            env: Default::default(),
        }];
        assert!(run.validate().is_ok(), "rejected safe {path} from {cwd}");
    }
}
