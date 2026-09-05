//! Terminal UI contracts for daemon-owned parallel Make execution.

#![cfg(unix)]

#[path = "make_parallel_dashboard_contract/failure.rs"]
mod failure;
#[path = "make_parallel_dashboard_contract/success.rs"]
mod success;
