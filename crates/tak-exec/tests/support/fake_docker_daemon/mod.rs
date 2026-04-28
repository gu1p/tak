#![allow(dead_code)]

mod build;
mod connection;
mod create;
mod daemon;
mod image_delete;
mod request;
mod response;
mod server;
mod state;
mod tar;
mod types;

pub use daemon::FakeDockerDaemon;
pub use types::{BuildRecord, CreateRecord, PullRecord};

const CONTAINER_ID: &str = "container-123";
const IMAGE_ID: &str = "sha256:test-image";
const LOG_MESSAGE: &[u8] = b"hello from container\n";
