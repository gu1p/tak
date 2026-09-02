use rusqlite::Connection;

use super::V2MixedOutputCluster;

impl V2MixedOutputCluster {
    pub fn output(&self, run_id: &str, path: &str) -> Vec<u8> {
        let artifact = self
            .store
            .output_manifest(run_id)
            .unwrap()
            .unwrap()
            .into_iter()
            .find(|artifact| artifact.path == path)
            .unwrap();
        self.store
            .output_chunk(&artifact.artifact_id, 0, 1024)
            .unwrap()
            .unwrap()
            .bytes
    }

    pub fn remote_attempt_count(&self, job_id: &str) -> u32 {
        Connection::open(self.worker.state_root.join("takd.sqlite"))
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM worker_v2_attempts WHERE job_id=?1",
                [job_id],
                |row| row.get(0),
            )
            .unwrap()
    }
}
