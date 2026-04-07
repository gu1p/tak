fn validate_remote_transport(transport: Option<RemoteTransportDef>) -> Result<RemoteTransportKind> {
    let Some(transport) = transport else {
        return Ok(RemoteTransportKind::Direct);
    };
    let kind = transport.kind.trim();
    if kind.is_empty() {
        bail!("execution Remote.transport.kind cannot be empty");
    }

    match kind {
        V1_TRANSPORT_DIRECT => Ok(RemoteTransportKind::Direct),
        V1_TRANSPORT_TOR => Ok(RemoteTransportKind::Tor),
        _ => bail!(
            "execution Remote.transport.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_DIRECT}` or `{V1_TRANSPORT_TOR}`"
        ),
    }
}

fn validate_remote_runtime(runtime: Option<RemoteRuntimeDef>) -> Result<Option<RemoteRuntimeSpec>> {
    let Some(runtime) = runtime else {
        return Ok(None);
    };

    let validated = validate_container_runtime_execution_spec(&runtime)
        .map_err(|err| anyhow!("execution Remote.{err}"))?;
    let image = validated.image;

    Ok(Some(RemoteRuntimeSpec::Containerized { image }))
}
