use std::thread;
use std::time::Duration;

use takd::SubmitAttemptStore;

use crate::support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use crate::support::remote_container::configure_fake_docker_env;
use crate::support::remote_output::test_context_with_runtime;

#[path = "remote_resource_admission_behavior/cancel.rs"]
mod cancel;
#[path = "remote_resource_admission_behavior/defaults_queue.rs"]
mod defaults_queue;
#[path = "remote_resource_admission_behavior/initial_sample.rs"]
mod initial_sample;
#[path = "remote_resource_admission_behavior/live_usage.rs"]
mod live_usage;
#[path = "remote_resource_admission_behavior/queue.rs"]
mod queue;
#[path = "remote_resource_admission_behavior/reservations.rs"]
mod reservations;
#[path = "remote_resource_admission_behavior/status.rs"]
mod status;
#[path = "remote_resource_admission_behavior/submit.rs"]
mod submit;
#[path = "remote_resource_admission_behavior/truthful_status.rs"]
mod truthful_status;
#[path = "remote_resource_admission_behavior/unlimited.rs"]
mod unlimited;

use status::{majority_memory_limits, status, task_events, wait_for_status, wait_for_task_event};
use submit::submit;
