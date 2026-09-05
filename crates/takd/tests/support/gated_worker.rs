use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, watch};
use tokio::task::{JoinHandle, JoinSet};

pub struct GatedWorker {
    pub addr: SocketAddr,
    pub started: Arc<Notify>,
    release: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl GatedWorker {
    pub async fn start(worker: SocketAddr) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let started = Arc::new(Notify::new());
        let notify = started.clone();
        let (release, gate) = watch::channel(false);
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (mut inbound, _) = accepted.unwrap();
                        let mut gate = gate.clone();
                        notify.notify_one();
                        connections.spawn(async move {
                            if gate.wait_for(|open| *open).await.is_err() { return; }
                            let mut outbound = TcpStream::connect(worker).await.unwrap();
                            let _ = tokio::io::copy_bidirectional(&mut inbound, &mut outbound).await;
                        });
                    }
                    Some(_) = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        });
        Self {
            addr,
            started,
            release,
            task,
        }
    }

    pub fn release(&self) {
        self.release.send(true).unwrap();
    }
}

impl Drop for GatedWorker {
    fn drop(&mut self) {
        self.task.abort();
    }
}
