use std::future::Future;

pub(super) struct ServerBackgroundTask {
    task: Option<tokio::task::JoinHandle<()>>,
}

impl ServerBackgroundTask {
    pub(super) fn spawn(future: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            task: Some(tokio::spawn(future)),
        }
    }

    pub(super) fn abort(&self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }

    pub(super) async fn shutdown(mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            let _ = task.await;
        }
    }
}

impl Drop for ServerBackgroundTask {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
