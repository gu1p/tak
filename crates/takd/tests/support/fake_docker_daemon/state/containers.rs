use std::collections::BTreeMap;

use super::super::{CreateRecord, DockerOperation};
use super::FakeDockerDaemonState;

mod transitions;

impl FakeDockerDaemonState {
    pub(in super::super) fn create_records(&self) -> Vec<CreateRecord> {
        self.create_records
            .lock()
            .expect("create records lock")
            .clone()
    }

    pub(in super::super) fn add_container(
        &self,
        container_id: &str,
        labels: BTreeMap<String, String>,
    ) {
        self.add_container_with_state(container_id, labels, "running");
    }

    pub(in super::super) fn add_container_with_state(
        &self,
        container_id: &str,
        labels: BTreeMap<String, String>,
        state: &str,
    ) {
        self.record_create(
            CreateRecord {
                container_id: container_id.to_string(),
                image: Some("alpine:3.20".to_string()),
                cmd: vec!["sleep".to_string(), "60".to_string()],
                user: None,
                working_dir: None,
                binds: Vec::new(),
                labels,
                env: Vec::new(),
                nano_cpus: None,
                state: state.to_string(),
            },
            0,
        );
    }

    pub(in super::super) fn container_summaries(&self) -> Vec<CreateRecord> {
        let removed = self
            .removed_containers
            .lock()
            .expect("removed containers lock")
            .clone();
        self.create_records
            .lock()
            .expect("create records lock")
            .iter()
            .filter(|record| !removed.contains(&record.container_id))
            .cloned()
            .collect()
    }

    pub(in super::super) fn record_create(&self, record: CreateRecord, exit_code: i64) {
        self.operations
            .lock()
            .expect("operations lock")
            .push(DockerOperation::Created(record.container_id.clone()));
        self.container_exit_codes
            .lock()
            .expect("container exit codes lock")
            .insert(record.container_id.clone(), exit_code);
        self.create_records
            .lock()
            .expect("create records lock")
            .push(record);
    }
}
