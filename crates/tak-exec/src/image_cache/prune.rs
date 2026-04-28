use bollard::Docker;
use bollard::errors::Error as BollardError;
use bollard::image::RemoveImageOptions;

pub(super) async fn remove_cached_image(docker: &Docker, image_ref: &str) -> bool {
    match docker
        .remove_image(
            image_ref,
            Some(RemoveImageOptions {
                force: true,
                noprune: false,
            }),
            None,
        )
        .await
    {
        Ok(_) => true,
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => true,
        Err(err) => {
            tracing::warn!("image cache prune skipped cached image {image_ref}: {err:#}");
            false
        }
    }
}
