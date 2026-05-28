use super::*;

pub(super) fn normalize_workspace_submit_glob(value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("invalid_submit_fields: outputs.glob cannot be empty");
    }
    if normalized.starts_with('@') {
        bail!("invalid_submit_fields: outputs repo anchors are not supported in V1");
    }
    if normalized.starts_with("//") || normalized.starts_with('/') {
        bail!("invalid_submit_fields: outputs glob must be workspace-relative");
    }
    if normalized.split('/').any(|segment| segment == "..") {
        bail!("invalid_submit_fields: outputs glob cannot escape workspace");
    }
    let normalized = normalized.replace('\\', "/");
    let mut builder = GitignoreBuilder::new(".");
    builder
        .add_line(None, &normalized)
        .map_err(|err| anyhow!("invalid_submit_fields: outputs.glob {err}"))?;
    Ok(normalized)
}
