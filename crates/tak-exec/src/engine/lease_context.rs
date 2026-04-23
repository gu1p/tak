use std::env;

use uuid::Uuid;

use super::RunOptions;

#[derive(Debug, Clone)]
pub(crate) struct LeaseContext {
    pub(crate) user: String,
    pub(crate) session_id: String,
}

impl LeaseContext {
    /// Builds a lease context using explicit options or environment-derived defaults.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn from_options(options: &RunOptions) -> Self {
        let user = options.user.clone().unwrap_or_else(|| {
            env::var("USER")
                .or_else(|_| env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string())
        });
        let session_id = options
            .session_id
            .clone()
            .unwrap_or_else(|| format!("tak-{}", Uuid::new_v4()));

        Self { user, session_id }
    }
}
