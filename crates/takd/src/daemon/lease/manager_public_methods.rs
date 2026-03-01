use super::*;

impl LeaseManager {
    /// Attempts to atomically acquire all requested needs for a lease request.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn acquire(&mut self, request: AcquireLeaseRequest) -> AcquireLeaseResponse {
        self.expire_leases();

        if let Some(existing) = self
            .pending
            .iter()
            .position(|pending| pending.request_id == request.request_id)
        {
            self.pending.remove(existing);
        }

        if !self.can_allocate(&request.needs) {
            return self.enqueue_pending_request(request);
        }

        self.allocate(&request.needs);
        let lease_id = Uuid::new_v4().to_string();
        let ttl_ms = request.ttl_ms.max(1_000);
        let lease_record = LeaseRecord {
            needs: request.needs.clone(),
            expires_at: Instant::now() + Duration::from_millis(ttl_ms),
            ttl_ms,
            request_id: request.request_id.clone(),
            task_label: request.task.label.clone(),
            user_name: request.client.user.clone(),
            pid: request.client.pid,
        };

        if let Err(err) = self.persist_acquired_lease(&lease_id, &lease_record) {
            self.deallocate(&request.needs);
            eprintln!("failed to persist acquired lease {lease_id}: {err}");
            return self.enqueue_pending_request(request);
        }

        self.leases.insert(lease_id.clone(), lease_record);
        AcquireLeaseResponse::LeaseGranted {
            lease: LeaseInfo {
                lease_id,
                ttl_ms,
                renew_after_ms: ttl_ms / 3,
            },
        }
    }

    fn enqueue_pending_request(&mut self, request: AcquireLeaseRequest) -> AcquireLeaseResponse {
        self.pending.push_back(request);
        AcquireLeaseResponse::LeasePending {
            pending: PendingInfo {
                queue_position: self.pending.len(),
            },
        }
    }

    fn persist_acquired_lease(&self, lease_id: &str, lease_record: &LeaseRecord) -> Result<()> {
        self.persist_active_lease(lease_id, lease_record)?;
        if let Err(err) = self.append_history("acquire", lease_id, lease_record) {
            if let Err(cleanup_err) = self.delete_active_lease(lease_id) {
                eprintln!(
                    "failed to rollback sqlite lease {lease_id} after history failure: {cleanup_err}"
                );
            }
            return Err(err);
        }
        Ok(())
    }

    /// Renews an existing lease by updating TTL and persisted expiry.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn renew(&mut self, lease_id: &str, ttl_ms: u64) -> Result<()> {
        self.expire_leases();
        let effective_ttl = ttl_ms.max(1_000);

        let updated_record = {
            let record = self
                .leases
                .get_mut(lease_id)
                .ok_or_else(|| anyhow!("lease {lease_id} does not exist"))?;

            record.ttl_ms = effective_ttl;
            record.expires_at = Instant::now() + Duration::from_millis(effective_ttl);
            record.clone()
        };

        self.persist_active_lease(lease_id, &updated_record)?;
        self.append_history("renew", lease_id, &updated_record)?;

        Ok(())
    }

    /// Releases an active lease and reclaims associated limiter usage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn release(&mut self, lease_id: &str) -> Result<()> {
        self.expire_leases();

        let record = self
            .leases
            .remove(lease_id)
            .ok_or_else(|| anyhow!("lease {lease_id} does not exist"))?;
        self.deallocate(&record.needs);
        self.delete_active_lease(lease_id)?;
        self.append_history("release", lease_id, &record)?;

        Ok(())
    }
}
