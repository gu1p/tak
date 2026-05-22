use tak_core::model::{ExecutionPlacementSpec, RemoteSelectionSpec};

use super::{load_task, policy_placements};

#[test]
fn execution_policy_accepts_explicit_sequential_remote_selection() {
    let task = load_task(
        r#"RUNTIME=Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512)); POLICY=Execution.FirstAvailable(placements=[Execution.Remote(container=RUNTIME, selection=RemoteSelection.Sequential())]); SPEC=module_spec(tasks=[task("check", steps=[cmd("true")], execution=POLICY)]); SPEC"#,
    );
    match &policy_placements(&task)[0] {
        ExecutionPlacementSpec::Remote(remote) => {
            assert!(matches!(remote.selection, RemoteSelectionSpec::Sequential))
        }
        other => panic!("expected remote placement, got {other:?}"),
    }
}

#[test]
fn execution_policy_accepts_explicit_shuffle_remote_selection() {
    let task = load_task(
        r#"RUNTIME=Container.Image("alpine:3.20", resources=Container.Resources(cpu_cores=1.0, memory_mb=512)); POLICY=Execution.FirstAvailable(placements=[Execution.Remote(container=RUNTIME, selection=RemoteSelection.Shuffle())]); SPEC=module_spec(tasks=[task("check", steps=[cmd("true")], execution=POLICY)]); SPEC"#,
    );
    match &policy_placements(&task)[0] {
        ExecutionPlacementSpec::Remote(remote) => {
            assert!(matches!(remote.selection, RemoteSelectionSpec::Shuffle))
        }
        other => panic!("expected remote placement, got {other:?}"),
    }
}
