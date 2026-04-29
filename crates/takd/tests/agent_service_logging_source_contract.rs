use std::path::PathBuf;

#[test]
fn service_log_formatter_keeps_verbose_event_context() {
    let source =
        std::fs::read_to_string(repo_root().join("crates/takd/src/logging.rs")).expect("read log");

    assert!(source.contains(".with_target(true)"), "{source}");
    assert!(source.contains(".with_thread_ids(true)"), "{source}");
    assert!(source.contains(".with_file(true)"), "{source}");
    assert!(source.contains(".with_line_number(true)"), "{source}");
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
