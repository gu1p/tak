use std::time::Duration;

use takd::{AttemptCoordinator, AttemptTransport};

pub async fn until<T: AttemptTransport + 'static>(
    coordinator: &mut AttemptCoordinator<T>,
    mut done: impl FnMut() -> bool,
) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !done() {
            coordinator.drive_once().await.unwrap();
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("coordinator did not make progress");
}
