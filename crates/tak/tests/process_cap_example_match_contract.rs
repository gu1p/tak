use std::{fs, path::Path};

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
}

#[test]
fn process_cap_guard_uses_a_fixture_specific_process_match() {
    let tasks =
        fs::read_to_string(repo_root().join("examples/medium/16_process_cap_guard/TASKS.py"))
            .expect("process-cap example TASKS.py");

    assert!(
        tasks.contains("match=\"tak-example-medium-16-simulator\""),
        "the process-cap fixture must not collide with unrelated host processes"
    );
    assert!(
        !tasks.contains("match=\"sim\""),
        "a generic process substring can permanently saturate the example on shared CI hosts"
    );
}
