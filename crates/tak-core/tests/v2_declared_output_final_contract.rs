use tak_core::v2::{ProducedOutput, ResolvedTaskUnit, WorkspaceEntry, resolve_final_outputs};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn final_outputs_use_targets_as_a_synthetic_sink_and_keep_unshadowed_ancestors() {
    let mut run = sample_run();
    let template = run.tasks[0].clone();
    run.tasks = [
        task(&template, "base", &[]),
        task(&template, "target", &["base"]),
    ]
    .into();
    run.targets = vec!["//:target".into()];
    let merged = resolve_final_outputs(
        &run,
        [
            output("base", "replaced", 'a'),
            output("base", "kept", 'c'),
            output("target", "replaced", 'b'),
        ],
    )
    .unwrap();
    assert_eq!(
        merged
            .outputs
            .iter()
            .map(|output| (output.entry.path.as_str(), output.payload))
            .collect::<Vec<_>>(),
        [("kept", "base"), ("replaced", "target")]
    );
}

#[test]
fn independent_targets_with_different_outputs_conflict_at_the_final_sink() {
    let mut run = sample_run();
    let template = run.tasks[0].clone();
    run.tasks = [task(&template, "left", &[]), task(&template, "right", &[])].into();
    run.targets = vec!["//:left".into(), "//:right".into()];

    let error = resolve_final_outputs(
        &run,
        [
            output("left", "dist/result", 'a'),
            output("right", "dist/result", 'b'),
        ],
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "independent producers conflict on declared output `dist/result` before `final run`"
    );
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

fn output(task: &'static str, path: &str, digest: char) -> ProducedOutput<&'static str> {
    ProducedOutput {
        producer_task_id: format!("//:{task}"),
        entry: WorkspaceEntry::file(path, false, 1, &digest.to_string().repeat(64)).unwrap(),
        payload: task,
    }
}
