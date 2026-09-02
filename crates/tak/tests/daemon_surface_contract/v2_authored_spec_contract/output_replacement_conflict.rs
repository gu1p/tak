use super::output_replacement_conflict_support::assert_replacement_preserves_checkout;

#[test]
fn file_and_symlink_replacements_report_all_changes_and_apply_nothing() {
    assert_replacement_preserves_checkout();
}
