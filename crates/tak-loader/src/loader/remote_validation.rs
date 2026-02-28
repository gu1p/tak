fn validate_remote_transport(
    transport: Option<RemoteTransportDef>,
) -> Result<(RemoteTransportKind, Option<String>)> {
    let Some(transport) = transport else {
        return Ok((RemoteTransportKind::DirectHttps, None));
    };

    let service_auth_env = validate_service_auth(transport.auth)?;
    let kind = transport.kind.trim();
    if kind.is_empty() {
        bail!("execution Remote.transport.kind cannot be empty");
    }

    match kind {
        V1_TRANSPORT_DIRECT_HTTPS => Ok((RemoteTransportKind::DirectHttps, service_auth_env)),
        V1_TRANSPORT_TOR => Ok((RemoteTransportKind::Tor, service_auth_env)),
        _ => bail!(
            "execution Remote.transport.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_DIRECT_HTTPS}` or `{V1_TRANSPORT_TOR}`"
        ),
    }
}

fn validate_service_auth(auth: Option<ServiceAuthDef>) -> Result<Option<String>> {
    let Some(auth) = auth else {
        return Ok(None);
    };

    let kind = auth.kind.trim();
    if kind != V1_TRANSPORT_AUTH_FROM_ENV {
        bail!(
            "execution Remote.transport.auth.kind `{kind}` is unsupported in V1; expected `{V1_TRANSPORT_AUTH_FROM_ENV}`"
        );
    }

    let env_name = auth
        .env_name
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    if env_name.is_empty() {
        bail!("execution Remote.transport.auth.env_name cannot be empty");
    }

    Ok(Some(env_name))
}

fn validate_remote_workspace(workspace: Option<RemoteWorkspaceDef>) -> Result<()> {
    let Some(workspace) = workspace else {
        return Ok(());
    };

    let transfer = workspace.transfer.trim();
    if transfer != V1_WORKSPACE_TRANSFER_MODE {
        bail!("execution Remote.workspace.transfer must be `{V1_WORKSPACE_TRANSFER_MODE}` in V1");
    }

    Ok(())
}

fn validate_remote_result(result: Option<RemoteResultDef>) -> Result<()> {
    let Some(result) = result else {
        return Ok(());
    };

    let sync = result.sync.trim();
    if sync != V1_RESULT_SYNC_MODE {
        bail!("execution Remote.result.sync must be `{V1_RESULT_SYNC_MODE}` in V1");
    }

    Ok(())
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
