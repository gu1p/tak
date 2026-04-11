pub fn normalize_container_image_reference(
    image: &str,
) -> Result<ContainerImageReference, ContainerImageReferenceError> {
    let trimmed = image.trim();
    if trimmed.is_empty() {
        return Err(ContainerImageReferenceError::EmptyReference);
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(ContainerImageReferenceError::ContainsWhitespace);
    }
    if trimmed.contains("://") {
        return Err(ContainerImageReferenceError::ContainsScheme);
    }

    let mut parts = trimmed.split('@');
    let image_name = parts.next().unwrap_or_default();
    let digest = parts.next();
    if parts.next().is_some() {
        return Err(ContainerImageReferenceError::MalformedDigest);
    }

    if image_name.is_empty() {
        return Err(ContainerImageReferenceError::EmptyReference);
    }

    let canonical_image = normalize_image_name_and_tag(image_name)?;
    let Some(digest) = digest else {
        return Ok(ContainerImageReference {
            canonical: canonical_image,
            digest_pinned: false,
        });
    };

    let (raw_algorithm, raw_hex) = digest
        .split_once(':')
        .ok_or(ContainerImageReferenceError::MalformedDigest)?;
    if raw_algorithm.is_empty() {
        return Err(ContainerImageReferenceError::EmptyDigestAlgorithm);
    }
    if raw_hex.is_empty() {
        return Err(ContainerImageReferenceError::EmptyDigest);
    }
    if !raw_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ContainerImageReferenceError::NonHexDigest);
    }

    let algorithm = raw_algorithm.to_ascii_lowercase();
    let digest_hex = raw_hex.to_ascii_lowercase();
    if algorithm == "sha256" && digest_hex.len() != 64 {
        return Err(ContainerImageReferenceError::InvalidSha256DigestLength);
    }

    Ok(ContainerImageReference {
        canonical: format!("{canonical_image}@{algorithm}:{digest_hex}"),
        digest_pinned: true,
    })
}

pub fn validate_container_runtime_execution_spec(
    runtime: &RemoteRuntimeDef,
) -> Result<ContainerRuntimeExecutionSpec, ContainerRuntimeExecutionSpecError> {
    let kind = runtime.kind.trim();
    if kind != "containerized" {
        return Err(ContainerRuntimeExecutionSpecError::UnsupportedKind {
            kind: kind.to_string(),
        });
    }

    let source = validate_container_runtime_source(runtime)?;

    let command = normalize_runtime_command(runtime.command.as_ref())?;
    let mounts = normalize_runtime_mounts(&runtime.mounts)?;
    let env = normalize_runtime_env(&runtime.env)?;
    let resource_limits = normalize_runtime_resource_limits(runtime.resource_limits.as_ref())?;

    Ok(ContainerRuntimeExecutionSpec {
        source,
        command,
        mounts,
        env,
        resource_limits,
    })
}

fn validate_container_runtime_source(
    runtime: &RemoteRuntimeDef,
) -> Result<ContainerRuntimeSourceInputSpec, ContainerRuntimeExecutionSpecError> {
    let image = runtime.image.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty());
    let dockerfile = runtime.dockerfile.as_ref();

    match (image, dockerfile) {
        (Some(_), Some(_)) => Err(ContainerRuntimeExecutionSpecError::MultipleSources),
        (None, None) => Err(ContainerRuntimeExecutionSpecError::MissingSource),
        (Some(image), None) => Ok(ContainerRuntimeSourceInputSpec::Image {
            image: normalize_container_image_reference(image)
                .map_err(ContainerRuntimeExecutionSpecError::InvalidImage)?
                .canonical,
        }),
        (None, Some(dockerfile)) => {
            let dockerfile = match dockerfile {
                PathInputDef::Path { value } if !value.trim().is_empty() => dockerfile.clone(),
                PathInputDef::Path { .. } => {
                    return Err(ContainerRuntimeExecutionSpecError::MissingDockerfile);
                }
            };
            if runtime
                .build_context
                .as_ref()
                .is_some_and(|path| matches!(path, PathInputDef::Path { value } if value.trim().is_empty()))
            {
                return Err(ContainerRuntimeExecutionSpecError::InvalidBuildContextPathType);
            }
            Ok(ContainerRuntimeSourceInputSpec::Dockerfile {
                dockerfile,
                build_context: runtime.build_context.clone(),
            })
        }
    }
}
