use super::*;

#[path = "dispatch/remote.rs"]
mod remote;

pub(super) async fn dispatch_request(
    request: Request,
    manager: &SharedLeaseManager,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
    tasks: &DaemonTaskHandles,
) -> Result<Response> {
    match request {
        Request::AcquireLease(payload) => {
            let request_id = payload.request_id.clone();

            if let Err(err) = ensure_valid_request(&payload) {
                return Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                });
            }

            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            let response = guard.acquire(payload);
            Ok(match response {
                AcquireLeaseResponse::LeaseGranted { lease } => {
                    Response::LeaseGranted { request_id, lease }
                }
                AcquireLeaseResponse::LeasePending { pending } => Response::LeasePending {
                    request_id,
                    pending,
                },
            })
        }
        Request::RenewLease(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            match guard.renew(&payload.lease_id, payload.ttl_ms) {
                Ok(()) => Ok(Response::LeaseRenewed {
                    request_id: payload.request_id,
                    ttl_ms: payload.ttl_ms,
                }),
                Err(err) => Ok(Response::Error {
                    request_id: payload.request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::ReleaseLease(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            match guard.release(&payload.lease_id) {
                Ok(()) => Ok(Response::LeaseReleased {
                    request_id: payload.request_id,
                }),
                Err(err) => Ok(Response::Error {
                    request_id: payload.request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::Status(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            Ok(Response::StatusSnapshot {
                request_id: payload.request_id,
                status: guard.status(),
            })
        }
        Request::PeersList(payload) => Ok(Response::PeersSnapshot {
            request_id: payload.request_id,
            peers: peers.snapshots(),
        }),
        Request::PeersEligible(payload) => Ok(Response::PeersSnapshot {
            request_id: payload.request_id,
            peers: peers.eligible(&payload.requirements),
        }),
        Request::PlaceRemote(payload) => {
            let eligible = peers.snapshots();
            match crate::daemon::peer_manager::first_placeable_or_error(
                &eligible,
                &payload.requirements,
            ) {
                Ok(peer) => remote::place_remote_task(payload, peer, peers, broker, tasks).await,
                Err(err) => Ok(Response::Error {
                    request_id: payload.request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::ForwardRemoteHttp(payload) => {
            remote::forward_remote_http(payload, peers, broker).await
        }
        Request::StreamTaskEvents(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => {
                    remote::stream_task_events(request_id, task, payload, peers, broker).await
                }
                Err(err) => Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::CancelTask(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => remote::cancel_task(request_id, task, payload, peers, broker).await,
                Err(err) => Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::GetTaskResult(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => remote::get_task_result(request_id, task, peers, broker).await,
                Err(err) => Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::GetOutputRange(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => {
                    remote::get_output_range(request_id, task, payload, peers, broker).await
                }
                Err(err) => Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                }),
            }
        }
    }
}
