#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use takd::agent::{TransportHealth, TransportState, read_transport_health};
use tokio::time::sleep;

pub async fn wait_for_transport_state(
    state_root: &Path,
    expected: TransportState,
) -> TransportHealth {
    for _ in 0..100 {
        if let Some(health) = read_transport_health(state_root).expect("read transport health")
            && health.transport_state == expected
        {
            return health;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for transport state {expected:?}");
}
