use super::state_types::{TaskActivity, TaskRow};

const LARGE_RUN_THRESHOLD: usize = 12;
const RECENT_SUCCESS_SLOTS: usize = 8;

pub(super) fn visible_rows<'a>(rows: &[&'a TaskRow]) -> (Vec<&'a TaskRow>, usize) {
    if rows.len() <= LARGE_RUN_THRESHOLD {
        return (rows.to_vec(), 0);
    }
    let recent_passed = rows
        .iter()
        .rev()
        .filter(|row| row.activity == TaskActivity::Passed)
        .take(RECENT_SUCCESS_SLOTS)
        .map(|row| row.label.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let visible = rows
        .iter()
        .copied()
        .filter(|row| row.activity != TaskActivity::Passed || recent_passed.contains(&row.label))
        .collect::<Vec<_>>();
    let hidden = rows.len().saturating_sub(visible.len());
    (visible, hidden)
}
