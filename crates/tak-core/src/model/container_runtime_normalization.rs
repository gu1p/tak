use super::*;

pub(crate) fn normalize_image_name_and_tag(
    image_name: &str,
) -> Result<String, ContainerImageReferenceError> {
    let last_slash = image_name.rfind('/');
    let last_colon = image_name.rfind(':');

    let split_tag = match (last_colon, last_slash) {
        (Some(colon), Some(slash)) => colon > slash,
        (Some(_), None) => true,
        (None, _) => false,
    };

    if split_tag {
        let colon = last_colon.ok_or(ContainerImageReferenceError::MalformedDigest)?;
        let repository = &image_name[..colon];
        let tag = &image_name[colon + 1..];
        if repository.is_empty() || tag.is_empty() {
            return Err(ContainerImageReferenceError::EmptyReference);
        }
        return Ok(format!("{}:{tag}", repository.to_ascii_lowercase()));
    }

    Ok(image_name.to_ascii_lowercase())
}

pub(crate) fn normalize_runtime_command(
    command: Option<&Vec<String>>,
) -> Result<Vec<String>, ContainerRuntimeExecutionSpecError> {
    let Some(command) = command else {
        return Ok(Vec::new());
    };
    if command.is_empty() {
        return Err(ContainerRuntimeExecutionSpecError::EmptyCommand);
    }

    let mut normalized = Vec::with_capacity(command.len());
    for (index, value) in command.iter().enumerate() {
        let argument = value.trim();
        if argument.is_empty() {
            return Err(ContainerRuntimeExecutionSpecError::EmptyCommandArg { index });
        }
        normalized.push(argument.to_string());
    }
    Ok(normalized)
}

pub(crate) fn normalize_runtime_mounts(
    mounts: &[ContainerMountDef],
) -> Result<Vec<ContainerMountSpec>, ContainerRuntimeExecutionSpecError> {
    let mut normalized = Vec::with_capacity(mounts.len());
    for (index, mount) in mounts.iter().enumerate() {
        let source = mount.source.trim();
        if source.is_empty() {
            return Err(ContainerRuntimeExecutionSpecError::EmptyMountSource { index });
        }
        let target = normalize_runtime_mount_target(&mount.target).ok_or_else(|| {
            ContainerRuntimeExecutionSpecError::InvalidMountTarget {
                index,
                target: mount.target.trim().to_string(),
            }
        })?;
        normalized.push(ContainerMountSpec {
            source: source.replace('\\', "/"),
            target,
            read_only: mount.read_only,
        });
    }

    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_runtime_mount_target(target: &str) -> Option<String> {
    let normalized = target.trim().replace('\\', "/");
    if normalized.is_empty() || !normalized.starts_with('/') {
        return None;
    }

    let mut segments = Vec::new();
    for segment in normalized.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return None,
            value => segments.push(value.to_string()),
        }
    }

    if segments.is_empty() {
        return Some("/".to_string());
    }
    Some(format!("/{}", segments.join("/")))
}

pub(crate) fn normalize_runtime_env(
    env: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, ContainerRuntimeExecutionSpecError> {
    let mut normalized = BTreeMap::new();
    for (key, value) in env {
        let key = key.trim();
        if !is_valid_runtime_env_key(key) {
            return Err(ContainerRuntimeExecutionSpecError::InvalidEnvKey {
                key: key.to_string(),
            });
        }
        if is_reserved_runtime_env_key(key) {
            return Err(ContainerRuntimeExecutionSpecError::ReservedEnvKey {
                key: key.to_string(),
            });
        }
        if value.contains('\0') {
            return Err(ContainerRuntimeExecutionSpecError::InvalidEnvValue {
                key: key.to_string(),
                value_preview: redact_runtime_env_value(key, value),
            });
        }
        normalized.insert(key.to_string(), value.clone());
    }
    Ok(normalized)
}

fn is_valid_runtime_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_uppercase() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn is_reserved_runtime_env_key(key: &str) -> bool {
    matches!(
        key,
        "TAK_RUNTIME"
            | "TAK_RUNTIME_ENGINE"
            | "TAK_RUNTIME_SOURCE"
            | "TAK_CONTAINER_IMAGE"
            | "TAK_REMOTE_RUNTIME"
            | "TAK_REMOTE_ENGINE"
            | "TAK_REMOTE_CONTAINER_IMAGE"
    )
}

fn redact_runtime_env_value(key: &str, value: &str) -> String {
    if is_sensitive_runtime_env_key(key) {
        return "<redacted>".to_string();
    }
    let escaped = value.replace('\0', "\\0");
    if escaped.len() <= 64 {
        return escaped;
    }
    format!("{}...", &escaped[..64])
}
