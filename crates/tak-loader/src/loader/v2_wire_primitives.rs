use tak_core::v2::{OutputSelector, Step};

use super::v2_wire as wire;

pub(super) fn convert_step(step: wire::Step) -> Step {
    match step {
        wire::Step::Cmd { argv, cwd, env } => Step::Cmd { argv, cwd, env },
        wire::Step::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => Step::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        },
    }
}

pub(super) fn convert_output(output: wire::Output) -> OutputSelector {
    match output {
        wire::Output::Path { value } => OutputSelector::Path { value },
        wire::Output::Glob { value } => OutputSelector::Glob { value },
    }
}
