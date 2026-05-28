#![cfg(test)]

#[path = "client_remotes_tests/tests/any_transport.rs"]
mod any_transport;
#[path = "client_remotes_tests/tests/node_capability.rs"]
mod node_capability;
#[path = "client_remotes_tests/support.rs"]
mod support;

pub(crate) use support::env_lock;
