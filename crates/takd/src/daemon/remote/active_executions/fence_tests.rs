use std::sync::TryLockError;

use anyhow::anyhow;

use super::SharedActiveExecutions;

#[test]
fn idle_operation_holds_the_registry_fence_until_quarantine_finishes() {
    let executions = SharedActiveExecutions::default();

    let quarantined = executions
        .try_when_idle(|| {
            assert!(matches!(
                executions.inner.try_lock(),
                Err(TryLockError::WouldBlock)
            ));
            Ok(Some("quarantined"))
        })
        .expect("run idle operation");

    assert_eq!(quarantined, Some("quarantined"));
    assert!(executions.inner.try_lock().is_ok());
}

#[test]
fn finish_callback_errors_still_unregister_the_execution() {
    let executions = SharedActiveExecutions::default();
    let _cancellation = executions
        .register("submit-1".into(), "run-1", 1)
        .expect("register execution");
    let _other_cancellation = executions
        .register("submit-2".into(), "run-2", 1)
        .expect("register second execution");

    let result: anyhow::Result<()> = executions.unregister_after_locked("submit-1", || {
        assert!(matches!(
            executions.inner.try_lock(),
            Err(TryLockError::WouldBlock)
        ));
        Err(anyhow!("touch failed"))
    });

    assert_eq!(result.expect_err("touch error").to_string(), "touch failed");
    assert_eq!(
        executions.keys().expect("execution keys"),
        vec!["submit-2".to_string()]
    );
}

impl SharedActiveExecutions {
    pub(in crate::daemon::remote) fn is_unlocked_for_test(&self) -> bool {
        match self.inner.try_lock() {
            Ok(_guard) => true,
            Err(TryLockError::WouldBlock) => false,
            Err(TryLockError::Poisoned(_)) => false,
        }
    }
}
