use anyhow::{Context, Result, bail};
use bollard::Docker;
use bollard::image::CreateImageOptions;
use futures::StreamExt;

use crate::container_engine::{ContainerEngine, engine_name, podman_socket_candidates_from_env};

#[derive(Debug)]
pub(crate) struct ContainerEngineClient {
    pub(crate) docker: Docker,
    pub(super) podman_wait_socket: Option<String>,
}

pub(crate) async fn connect_container_engine(
    engine: ContainerEngine,
) -> Result<ContainerEngineClient> {
    match engine {
        ContainerEngine::Docker => {
            let docker = Docker::connect_with_local_defaults().context(
                "infra error: container lifecycle start failed: docker client connect failed",
            )?;
            docker.ping().await.with_context(|| {
                format!(
                    "infra error: container lifecycle start failed: {} ping failed",
                    engine_name(engine)
                )
            })?;
            Ok(ContainerEngineClient {
                docker,
                podman_wait_socket: None,
            })
        }
        ContainerEngine::Podman => connect_podman_client().await,
    }
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
    bail!("infra error: container lifecycle start failed: no podman socket available");
}

pub(super) async fn ensure_container_image(docker: &Docker, image: &str) -> Result<()> {
    match docker.inspect_image(image).await {
        Ok(_) => return Ok(()),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {}
        Err(err) => {
            return Err(err)
                .context("infra error: container lifecycle pull failed: inspect image failed");
        }
    }

    pull_container_image(docker, image).await
}

pub(super) async fn pull_container_image(docker: &Docker, image: &str) -> Result<()> {
    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: image.to_string(),
            ..Default::default()
        }),
        None,
        None,
    );
    while let Some(item) = stream.next().await {
        item.context("infra error: container lifecycle pull failed")?;
    }
    Ok(())
}
