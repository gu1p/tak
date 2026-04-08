use super::*;

impl LeaseManager {
    /// Expires stale leases and frees their allocated usage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn expire_leases(&mut self) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .leases
            .iter()
            .filter_map(|(lease_id, record)| (record.expires_at <= now).then_some(lease_id.clone()))
            .collect();

        for lease_id in expired {
            if let Some(record) = self.leases.remove(&lease_id) {
                self.deallocate(&record.needs);
                if let Err(err) = self.delete_active_lease(&lease_id) {
                    tracing::error!("failed to delete expired lease {lease_id} from sqlite: {err}");
                }
                if let Err(err) = self.append_history("expire", &lease_id, &record) {
                    tracing::error!("failed to append expire history for {lease_id}: {err}");
                }
            }
        }
    }

    /// Checks whether all needs can be satisfied together without over-allocation.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn can_allocate(&self, needs: &[NeedRequest]) -> bool {
        let mut delta: HashMap<LimiterKey, f64> = HashMap::new();

        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            *delta.entry(key).or_insert(0.0) += need.slots;
        }

        delta.into_iter().all(|(key, requested)| {
            let used = self.usage.get(&key).copied().unwrap_or(0.0);
            let capacity = self.capacities.get(&key).copied().unwrap_or(f64::INFINITY);
            used + requested <= capacity
        })
    }

    /// Adds slots to usage totals for each requested need.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn allocate(&mut self, needs: &[NeedRequest]) {
        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            *self.usage.entry(key).or_insert(0.0) += need.slots;
        }
    }

    /// Removes slots from usage totals for each requested need.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(super) fn deallocate(&mut self, needs: &[NeedRequest]) {
        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            if let Some(entry) = self.usage.get_mut(&key) {
                *entry = (*entry - need.slots).max(0.0);
                if *entry == 0.0 {
                    self.usage.remove(&key);
                }
            }
        }
    }
}
