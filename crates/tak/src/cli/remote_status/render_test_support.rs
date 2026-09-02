#![cfg(test)]

#[path = "render_test_support/dashboard.rs"]
mod dashboard;
#[path = "render_test_support/remotes.rs"]
mod remotes;
#[path = "render_test_support/status.rs"]
mod status;

pub(super) use dashboard::{render_dashboard_buffer, render_dashboard_text, style_for_text};
pub(super) use remotes::{error_result, ok_result, remote, warning_result};
