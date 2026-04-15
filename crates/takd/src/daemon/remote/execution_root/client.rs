use anyhow::{Context, Result, bail};
use bollard::Docker;
use bollard::container::{RemoveContainerOptions, WaitContainerOptions};
use bollard::image::CreateImageOptions;
use futures::StreamExt;

use crate::daemon::transport::ContainerEngine;

use super::PROBE_IMAGE;
use super::podman::{podman_socket_candidates_from_env, wait_for_container_exit_code_via_cli};

#[derive(Debug)]
pub(super) struct ContainerEngineClient {
    pub(super) docker: Docker,
    pub(super) podman_wait_socket: Option<String>,
}

pub(super) async fn connect_container_engine(
    engine: ContainerEngine,
) -> Result<ContainerEngineClient> {
    match engine {
        ContainerEngine::Docker => {
            let docker = Docker::connect_with_local_defaults()
                .context("docker client connect failed during exec-root probe")?;
            docker
                .ping()
                .await
                .context("docker ping failed during exec-root probe")?;
            Ok(ContainerEngineClient {
                docker,
                podman_wait_socket: None,
            })
        }
        ContainerEngine::Podman => connect_podman_client().await,
    }
}

pub(super) async fn ensure_container_image(docker: &Docker) -> Result<()> {
    match docker.inspect_image(PROBE_IMAGE).await {
        Ok(_) => return Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(err) => {
            return Err(err).context("inspect image failed during exec-root probe");
        }
    }

    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: PROBE_IMAGE.to_string(),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(item) = stream.next().await {
        item.context("pull image failed during exec-root probe")?;
    }
    Ok(())
}

pub(super) async fn wait_for_container_exit_code(
    client: &ContainerEngineClient,
    engine: ContainerEngine,
    container_name: &str,
) -> Result<i32> {
    match engine {
        ContainerEngine::Docker => {
            wait_for_container_exit_code_via_api(&client.docker, container_name).await
        }
        ContainerEngine::Podman => {
            wait_for_container_exit_code_via_cli(
                client.podman_wait_socket.as_deref(),
                container_name,
            )
            .await
        }
    }
}

pub(super) async fn cleanup_container(docker: &Docker, container_name: &str) -> Result<()> {
    let _ = docker
        .remove_container(
            container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await;
    Ok(())
}

async fn connect_podman_client() -> Result<ContainerEngineClient> {
    for socket in podman_socket_candidates_from_env() {
        let socket_path = socket.strip_prefix("unix://").unwrap_or(socket.as_str());
        let Ok(client) = Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)
        else {
            continue;
        };
        if client.ping().await.is_ok() {
            return Ok(ContainerEngineClient {
                docker: client,
                podman_wait_socket: Some(socket),
            });
        }
    }

    bail!("no podman socket available during exec-root probe");
}

async fn wait_for_container_exit_code_via_api(
    docker: &Docker,
    container_name: &str,
) -> Result<i32> {
    let mut stream = docker.wait_container(container_name, None::<WaitContainerOptions<String>>);
    let Some(result) = stream.next().await else {
        bail!("wait stream ended unexpectedly during exec-root probe");
    };
    let result = result.context("waiting for probe container failed")?;
    Ok(i32::try_from(result.status_code).unwrap_or(1))
}
