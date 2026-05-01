#[test]
fn task_spec_stub_includes_cascade_execution_field() {
    let stubs = include_str!("../src/loader/dsl_stubs.pyi");
    let task_spec = stubs
        .split("class TaskSpec(TypedDict):")
        .nth(1)
        .expect("TaskSpec stub")
        .split("# Top-level TASKS.py module payload")
        .next()
        .expect("TaskSpec section");

    assert!(
        task_spec.contains("cascade_execution: bool"),
        "TaskSpec stub should expose cascade_execution:\n{task_spec}"
    );
}
