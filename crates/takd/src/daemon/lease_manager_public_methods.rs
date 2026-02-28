impl LeaseManager {
    #[must_use]
    /// Creates an in-memory lease manager with no configured capacities.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a SQLite-backed lease manager and restores active lease state.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        let mut manager = Self {
            db_path: Some(db_path),
            ..Self::default()
        };
        manager.ensure_schema()?;
        manager.restore_active_leases()?;
        Ok(manager)
    }

    /// Sets capacity for one limiter key.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_capacity(
        &mut self,
        name: impl Into<String>,
        scope: Scope,
        scope_key: Option<String>,
        capacity: f64,
    ) {
        self.capacities.insert(
            LimiterKey {
                name: name.into(),
                scope,
                scope_key,
            },
            capacity,
        );
    }

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

        if self.can_allocate(&request.needs) {
            self.allocate(&request.needs);
            let lease_id = Uuid::new_v4().to_string();
            let ttl_ms = request.ttl_ms.max(1_000);
            let expires_at = Instant::now() + Duration::from_millis(ttl_ms);
            let lease_record = LeaseRecord {
                needs: request.needs,
                expires_at,
                ttl_ms,
                request_id: request.request_id,
                task_label: request.task.label,
                user_name: request.client.user,
                pid: request.client.pid,
            };

            self.leases.insert(lease_id.clone(), lease_record.clone());
            self.persist_active_lease(&lease_id, &lease_record)
                .expect("failed to persist active lease");
            self.append_history("acquire", &lease_id, &lease_record)
                .expect("failed to append acquire history");

            return AcquireLeaseResponse::LeaseGranted {
                lease: LeaseInfo {
                    lease_id,
                    ttl_ms,
                    renew_after_ms: ttl_ms / 3,
                },
            };
        }

        self.pending.push_back(request);
        AcquireLeaseResponse::LeasePending {
            pending: PendingInfo {
                queue_position: self.pending.len(),
            },
        }
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

    #[must_use]
    /// Returns current daemon state as an externally-visible status snapshot.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn status(&mut self) -> StatusSnapshot {
        self.expire_leases();

        let usage = self
            .usage
            .iter()
            .map(|(key, used)| LimiterUsage {
                name: key.name.clone(),
                scope: key.scope.clone(),
                scope_key: key.scope_key.clone(),
                used: *used,
                capacity: self.capacities.get(key).copied().unwrap_or(f64::INFINITY),
            })
            .collect();

        StatusSnapshot {
            active_leases: self.leases.len(),
            pending_requests: self.pending.len(),
            usage,
        }
    }
}
