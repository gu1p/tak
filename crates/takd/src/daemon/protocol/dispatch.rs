use super::*;

#[path = "dispatch/remote.rs"]
mod remote;

// How long `PlaceRemote` waits for a matching peer to become connected before
// giving up. Steady-state peers are already warm so this is usually a no-op;
// it only bites the first submit after startup or a reconnect. Generous enough
// to cover a cold onion dial completing in the heartbeat loop.
const DEFAULT_PLACE_REMOTE_WAIT_MS: u64 = 20_000;

fn place_remote_wait_timeout() -> std::time::Duration {
    std::env::var("TAKD_PLACE_REMOTE_WAIT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(std::time::Duration::from_millis)
        .unwrap_or_else(|| std::time::Duration::from_millis(DEFAULT_PLACE_REMOTE_WAIT_MS))
}

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
                return Ok(Response::error(request_id, err.to_string()));
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
                Err(err) => Ok(Response::error(payload.request_id, err.to_string())),
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
                Err(err) => Ok(Response::error(payload.request_id, err.to_string())),
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
        Request::PeersEligible(payload) => {
            // Match submit placement: prefer warm peers, but allow cold-dial
            // streaming to a still-Connecting peer.
            peers
                .wait_for_placeable_peer(&payload.requirements, place_remote_wait_timeout())
                .await;
            Ok(Response::PeersSnapshot {
                request_id: payload.request_id,
                peers: peers.placeable(&payload.requirements),
            })
        }
        Request::PlaceRemote(payload) => {
            // Give a just-configured or reconnecting peer a brief moment to warm
            // up so the submit lands on an already-open connection rather than
            // forcing a cold onion dial — the bridge should already be connected.
            peers
                .wait_for_placeable_peer(&payload.requirements, place_remote_wait_timeout())
                .await;
            let preferred_peer = payload.preferred_node_id.as_ref().and_then(|node_id| {
                peers
                    .placeable(&payload.requirements)
                    .into_iter()
                    .find(|peer| &peer.node_id == node_id)
            });
            let selected_peer = preferred_peer.map(Ok).unwrap_or_else(|| {
                peers.select_placeable(crate::daemon::peer_manager::PeerPlacementRequest {
                    requirements: &payload.requirements,
                    selection: payload.selection,
                    task_run_id: &payload.task_run_id,
                    attempt: payload.attempt,
                })
            });
            match selected_peer {
                Ok(peer) => {
                    tracing::info!(
                        task_run_id = %payload.task_run_id,
                        attempt = payload.attempt,
                        node_id = %peer.node_id,
                        endpoint = %peer.endpoint,
                        state = peer.state.as_str(),
                        "placing remote task through Tor peer"
                    );
                    remote::place_remote_task(payload, peer, peers, broker, tasks).await
                }
                Err(err) => {
                    tracing::warn!(
                        task_run_id = %payload.task_run_id,
                        attempt = payload.attempt,
                        error = %err,
                        "remote placement failed"
                    );
                    Ok(Response::classified_error(
                        payload.request_id,
                        err.to_string(),
                        err.code(),
                        err.is_retryable(),
                    ))
                }
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
                Err(err) => Ok(Response::error(request_id, err.to_string())),
            }
        }
        Request::CancelTask(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => remote::cancel_task(request_id, task, payload, peers, broker).await,
                Err(err) => Ok(Response::error(request_id, err.to_string())),
            }
        }
        Request::GetTaskResult(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => remote::get_task_result(request_id, task, peers, broker).await,
                Err(err) => Ok(Response::error(request_id, err.to_string())),
            }
        }
        Request::GetOutputRange(payload) => {
            let request_id = payload.request_id.clone();
            match tasks.resolve(&payload.task_handle) {
                Ok(task) => {
                    remote::get_output_range(request_id, task, payload, peers, broker).await
                }
                Err(err) => Ok(Response::error(request_id, err.to_string())),
            }
        }
    }
}
