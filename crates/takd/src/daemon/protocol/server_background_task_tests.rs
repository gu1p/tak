use std::future::pending;
use std::time::Duration;

use tokio::sync::oneshot;

use super::server_background_task::ServerBackgroundTask;

struct DropSignal(Option<oneshot::Sender<()>>);

impl Drop for DropSignal {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[tokio::test]
async fn server_background_task_is_cancelled_when_owner_drops() {
    let (started_tx, started_rx) = oneshot::channel();
    let (dropped_tx, dropped_rx) = oneshot::channel();
    let task = ServerBackgroundTask::spawn(async move {
        let _drop_signal = DropSignal(Some(dropped_tx));
        let _ = started_tx.send(());
        pending::<()>().await;
    });

    started_rx.await.expect("background task should start");
    drop(task);

    tokio::time::timeout(Duration::from_secs(1), dropped_rx)
        .await
        .expect("owned background task should stop promptly")
        .expect("background task should report cancellation");
}
