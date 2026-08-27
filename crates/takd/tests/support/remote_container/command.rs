use tak_proto::{CmdStep, Step, step};

pub(super) fn command_step(command: &str) -> Step {
    Step {
        kind: Some(step::Kind::Cmd(CmdStep {
            argv: vec!["sh".to_string(), "-c".to_string(), command.to_string()],
            cwd: None,
            env: Default::default(),
        })),
    }
}
