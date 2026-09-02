use std::fs;
use std::path::PathBuf;

#[test]
fn v2_evaluation_tolerates_descheduling_without_dropping_resource_caps() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/loader/v2_module_eval.rs"),
    )
    .expect("read v2 evaluator");

    assert!(
        source.contains("max_duration(Duration::from_secs(30))"),
        "DSL evaluation needs enough wall-clock headroom for loaded workers"
    );
    assert!(source.contains("max_memory(64 * 1024 * 1024)"));
    assert!(source.contains("max_allocations(200_000)"));
}
