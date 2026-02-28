pub struct LoadOptions {
    pub enable_type_check: bool,
    pub project_id: Option<String>,
}

impl Default for LoadOptions {
    /// Creates loader options with type checking enabled and no forced project id.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            enable_type_check: true,
            project_id: None,
        }
    }
}
