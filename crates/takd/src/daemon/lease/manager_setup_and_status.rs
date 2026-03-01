use super::*;

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
