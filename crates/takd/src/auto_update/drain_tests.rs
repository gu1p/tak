use super::drain::DrainOutcome;

#[test]
fn deadline_never_authorizes_replacing_a_worker_binary() {
    assert!(!DrainOutcome::DeadlineExceeded.allows_replacement());
    assert!(DrainOutcome::Idle.allows_replacement());
}
