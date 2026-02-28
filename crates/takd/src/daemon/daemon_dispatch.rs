fn dispatch_request(request: Request, manager: &SharedLeaseManager) -> Result<Response> {
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
        Request::RunTasks(_) => {
            bail!("run tasks request must be handled by async client stream handler")
        }
    }
}

