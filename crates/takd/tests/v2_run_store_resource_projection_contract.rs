use tak_core::v2::{ContainerSource, RuntimeResources, TaskRuntime};
use takd::RunStore;

use crate::support::v2_run::submission;

#[test]
fn daemon_store_rejects_container_work_that_would_bypass_its_resource_reservation() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut submitted = submission("resource-mismatch", "secret");
    submitted.run.tasks[0].runtime = Some(
        TaskRuntime::configured_container(
            ContainerSource::Image {
                image: "alpine:3.20".into(),
            },
            vec![],
            Default::default(),
            Some(RuntimeResources {
                cpu_millis: 2_000,
                memory_bytes: 2 * 1024 * 1024 * 1024,
            }),
        )
        .unwrap(),
    );

    let error = store.submit(&submitted, "alice").unwrap_err().to_string();

    assert!(error.contains("resources"), "{error}");
    assert!(store.list_runs().unwrap().is_empty());
}
