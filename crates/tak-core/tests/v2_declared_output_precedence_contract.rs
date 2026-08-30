use tak_core::v2::{ProducedOutput, ResolvedTaskUnit, WorkspaceEntry, resolve_dependency_outputs};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn nearest_producer_on_a_dependency_chain_replaces_earlier_output() {
    let run = graph(&[("a", &[]), ("b", &["a"]), ("c", &["b"])], &["c"]);
    let merged = resolve_dependency_outputs(
        &run,
        "//:c",
        [
            output("a", "result", 'a', false),
            output("b", "result", 'b', false),
        ],
    )
    .unwrap();
    assert_eq!(merged.outputs.len(), 1);
    assert_eq!(merged.outputs[0].payload, "b");
    assert_eq!(merged.outputs[0].producers, ["//:b"]);
}

#[test]
fn identical_independent_outputs_coalesce_but_metadata_differences_conflict() {
    let run = graph(
        &[
            ("root", &[]),
            ("left", &["root"]),
            ("right", &["root"]),
            ("join", &["left", "right"]),
        ],
        &["join"],
    );
    let identical = resolve_dependency_outputs(
        &run,
        "//:join",
        [
            output("left", "same", 'a', false),
            output("right", "same", 'a', false),
        ],
    )
    .unwrap();
    assert_eq!(identical.outputs[0].producers, ["//:left", "//:right"]);
    let conflict = resolve_dependency_outputs(
        &run,
        "//:join",
        [
            output("left", "same", 'a', false),
            output("right", "same", 'a', true),
        ],
    )
    .unwrap_err();
    assert!(conflict.to_string().contains("same"));
    assert!(conflict.to_string().contains("independent"));
}

fn graph(spec: &[(&str, &[&str])], targets: &[&str]) -> tak_core::v2::ResolvedRun {
    let mut run = sample_run();
    let template = run.tasks[0].clone();
    run.tasks = spec
        .iter()
        .map(|(name, dependencies)| ResolvedTaskUnit {
            task_id: format!("//:{name}"),
            job_id: format!("job-{name}"),
            dependencies: dependencies
                .iter()
                .map(|name| format!("//:{name}"))
                .collect(),
            ..template.clone()
        })
        .collect();
    run.targets = targets.iter().map(|name| format!("//:{name}")).collect();
    run
}

fn output(
    task: &'static str,
    path: &str,
    digest: char,
    executable: bool,
) -> ProducedOutput<&'static str> {
    ProducedOutput {
        producer_task_id: format!("//:{task}"),
        entry: WorkspaceEntry::file(path, executable, 1, &digest.to_string().repeat(64)).unwrap(),
        payload: task,
    }
}
