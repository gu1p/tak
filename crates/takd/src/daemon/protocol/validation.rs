use super::*;

/// Performs protocol-level validation for acquire-lease requests.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn ensure_valid_request(request: &AcquireLeaseRequest) -> Result<()> {
    if request.ttl_ms == 0 {
        bail!("ttl_ms must be positive");
    }
    if request.needs.is_empty() {
        bail!("at least one need must be provided");
    }
    Ok(())
}
