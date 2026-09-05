pub(in crate::cli) mod fallback;
mod input;
mod model;
mod navigation;
mod render;
mod runtime;
mod terminal;

pub(super) use fallback::{
    attempt_or_disable, disable_after_error, input_or_disable, start_or_disable,
};
pub(super) use model::DashboardSeed;
pub(super) use runtime::RunDashboard;

#[cfg(test)]
mod attempt_replay_bdd_tests;
#[cfg(test)]
mod attempt_tests;
#[cfg(test)]
mod cancellation_notice_tests;
#[cfg(test)]
mod cancellation_tests;
#[cfg(test)]
mod clarity_bdd_tests;
#[cfg(test)]
mod compact_layout_bdd_tests;
#[cfg(test)]
mod diagnostic_tests;
#[cfg(test)]
mod failure_visibility_bdd_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod interrupt_key_tests;
#[cfg(test)]
mod interrupt_repeat_bdd_tests;
#[cfg(test)]
mod model_queue_tests;
#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod narrow_operational_bdd_tests;
#[cfg(test)]
mod navigation_bdd_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod newline_output_bdd_tests;
#[cfg(test)]
mod node_lane_bdd_tests;
#[cfg(test)]
mod node_lane_tests;
#[cfg(test)]
mod output_metadata_safety_tests;
#[cfg(test)]
mod output_safety_tests;
#[cfg(test)]
mod queue_truth_tests;
#[cfg(test)]
mod render_test_support;
#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod responsive_tests;
#[cfg(test)]
mod runtime_visibility_tests;
#[cfg(test)]
mod staged_snapshot_bdd_tests;
#[cfg(test)]
mod terminal_cleanup_tests;
#[cfg(test)]
mod terminal_retry_tests;
#[cfg(test)]
mod terminal_tests;
#[cfg(test)]
mod test_support;
