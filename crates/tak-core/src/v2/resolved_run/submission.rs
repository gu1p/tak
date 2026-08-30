use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{EnvironmentValue, ResolvedRun, ResolvedRunError};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunSubmission {
    pub idempotency_key: String,
    pub run: ResolvedRun,
    pub environment_values: Vec<EnvironmentValue>,
}

impl RunSubmission {
    pub fn new(
        idempotency_key: impl Into<String>,
        run: ResolvedRun,
        mut environment_values: Vec<EnvironmentValue>,
    ) -> Result<Self, ResolvedRunError> {
        environment_values.sort_by(|left, right| left.name.cmp(&right.name));
        let submission = Self {
            idempotency_key: idempotency_key.into(),
            run,
            environment_values,
        };
        submission.validate()?;
        Ok(submission)
    }

    pub fn validate(&self) -> Result<(), ResolvedRunError> {
        self.run.validate()?;
        super::validation::validate_identifier("idempotency key", &self.idempotency_key)?;
        let requested = self
            .run
            .jobs
            .iter()
            .flat_map(|job| job.pass_env_names.iter().cloned())
            .collect::<BTreeSet<_>>();
        let supplied = self
            .environment_values
            .iter()
            .map(|entry| entry.name.clone())
            .collect::<Vec<_>>();
        let unique = supplied.iter().cloned().collect::<BTreeSet<_>>();
        if unique.len() != supplied.len() {
            return Err(ResolvedRunError::new("duplicate environment value name"));
        }
        if unique != requested {
            return Err(ResolvedRunError::new(
                "submitted environment values do not match requested names",
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn request_digest(&self) -> String {
        let payload = (&self.run, &self.environment_values);
        let bytes = serde_json::to_vec(&payload).expect("resolved submission is serializable");
        format!("{:x}", Sha256::digest(bytes))
    }
}

impl Debug for RunSubmission {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RunSubmission")
            .field("idempotency_key", &self.idempotency_key)
            .field("project_id", &self.run.project_id)
            .field("task_count", &self.run.tasks.len())
            .field("environment_values", &"<redacted>")
            .finish()
    }
}
