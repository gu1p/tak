use tak_core::v2::{ProducedOutput, ResolvedTaskUnit, WorkspaceEntry, resolve_dependency_outputs};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn a_shadowed_ancestor_is_removed_before_comparing_independent_frontier_outputs() {
    let mut run = sample_run();
    let template = run.tasks[0].clone();
    run.tasks = [
        task(&template, "join", &["b", "c"]),
        task(&template, "c", &[]),
        task(&template, "b", &["a"]),
        task(&template, "a", &[]),
    ]
    .into();
    let conflict = resolve_dependency_outputs(
        &run,
        "//:join",
        [output("c", 'c'), output("a", 'a'), output("b", 'b')],
    )
    .unwrap_err();
    assert!(conflict.to_string().contains("independent"));
    let merged = resolve_dependency_outputs(
        &run,
        "//:join",
        [output("c", 'b'), output("a", 'a'), output("b", 'b')],
    )
    .unwrap();
    assert_eq!(merged.outputs[0].payload, "b");
    assert_eq!(merged.outputs[0].producers, ["//:b", "//:c"]);
}

fn task(template: &ResolvedTaskUnit, name: &str, dependencies: &[&str]) -> ResolvedTaskUnit {
    ResolvedTaskUnit {
        task_id: format!("//:{name}"),
        job_id: format!("job-{name}"),
        dependencies: dependencies
            .iter()
            .map(|name| format!("//:{name}"))
            .collect(),
        ..template.clone()
    }
}

fn output(task: &'static str, digest: char) -> ProducedOutput<&'static str> {
    ProducedOutput {
        producer_task_id: format!("//:{task}"),
        entry: WorkspaceEntry::file("same", false, 1, &digest.to_string().repeat(64)).unwrap(),
        payload: task,
    }
}
