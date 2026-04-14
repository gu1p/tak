use futures::StreamExt;
use tokio::sync::mpsc;
use tor_cell::relaycell::msg::Connected;

use crate::daemon::remote::{SubmitAttemptStore, handle_remote_v1_http_stream};

use super::monitor::TorHealthEvent;

pub(super) fn spawn_rend_request(
    rend_request: tor_hsservice::RendRequest,
    store: SubmitAttemptStore,
    context: crate::daemon::remote::RemoteNodeContext,
    health_tx: mpsc::UnboundedSender<TorHealthEvent>,
) {
    std::mem::drop(tokio::spawn(async move {
        let accepted = rend_request.accept().await;
        let mut stream_requests = match accepted {
            Ok(stream_requests) => stream_requests,
            Err(err) => {
                let message = format!("rendezvous accept failed: {err}");
                let _ = health_tx.send(TorHealthEvent::Failure(message.clone()));
                tracing::error!("takd onion service {message}");
                return;
            }
        };
        while let Some(stream_request) = stream_requests.next().await {
            match stream_request.accept(Connected::new_empty()).await {
                Ok(mut stream) => {
                    handle_accepted_stream_side_effects(&context);
                    let store = store.clone();
                    let context = context.clone();
                    std::mem::drop(tokio::spawn(async move {
                        if let Err(err) =
                            handle_remote_v1_http_stream(&mut stream, &store, &context).await
                        {
                            tracing::error!("takd onion service stream handling failed: {err}");
                        }
                    }));
                }
                Err(err) => {
                    let message = format!("stream accept failed: {err}");
                    let _ = health_tx.send(TorHealthEvent::Failure(message.clone()));
                    tracing::error!("takd onion service {message}");
                }
            }
        }
    }));
}

pub(crate) fn handle_accepted_stream_side_effects(
    _context: &crate::daemon::remote::RemoteNodeContext,
) {
    // Accepted client streams are observational only. Transport readiness should
    // advance only from takd's self-probe so client requests cannot clear a
    // recovering state before `/v1/node/info` or `tak remote status` observes it.
}
