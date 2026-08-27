use super::pressure::PressureState;
use super::record_recovery_after_engine_work;
use crate::daemon::remote::http_server_test_support::node_context;
use crate::daemon::remote::resource_pressure_controller::ResourcePressureSnapshot;

#[test]
fn normal_deadband_after_restart_preserves_shared_pressure_episode() {
    let context = node_context();
    let pressure = ResourcePressureSnapshot::pressure(1_234);
    context
        .set_resource_pressure_snapshot(pressure.clone())
        .expect("store pressure episode");

    record_recovery_after_engine_work(&context, PressureState::Normal, true)
        .expect("record deadband sample");

    assert_eq!(
        context
            .resource_pressure_snapshot()
            .expect("load pressure episode"),
        pressure
    );
}
