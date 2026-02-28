use std::collections::HashSet;

use tak_core::model::TaskLabel;

use crate::RunSummary;

/// Returns the set of labels included in a run summary.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn target_set_from_summary(summary: &RunSummary) -> HashSet<TaskLabel> {
    summary.results.keys().cloned().collect()
}
