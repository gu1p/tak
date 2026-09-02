use tak_core::model::RemoteSelectionSpec;
use tak_core::v2::RemoteSelection;

use super::selection;

#[test]
fn intermediate_selection_preserves_each_v2_strategy() {
    assert_eq!(
        selection(RemoteSelectionSpec::Balanced).expect("balanced selection"),
        RemoteSelection::Balanced
    );
    assert_eq!(
        selection(RemoteSelectionSpec::RoundRobin).expect("round-robin selection"),
        RemoteSelection::RoundRobin
    );
    assert_eq!(
        selection(RemoteSelectionSpec::Sequential).expect("sequential selection"),
        RemoteSelection::Sequential
    );
}
