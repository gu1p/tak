use super::super::FakeDockerDaemonState;

pub(in crate::support::fake_docker_daemon) enum ImageDeleteResult {
    Removed,
    NotFound,
    Failed(u16),
}

impl FakeDockerDaemonState {
    pub(in crate::support::fake_docker_daemon) fn delete_image(
        &self,
        image: &str,
    ) -> ImageDeleteResult {
        self.image_removal_attempts
            .lock()
            .expect("image removal attempts lock")
            .push(image.to_string());
        if let Some(status_code) = *self
            .image_removal_failure_status
            .lock()
            .expect("image removal failure status lock")
        {
            return ImageDeleteResult::Failed(status_code);
        }
        if self
            .image_refs
            .lock()
            .expect("image refs lock")
            .remove(image)
            .is_some()
        {
            return ImageDeleteResult::Removed;
        }
        if self
            .images
            .lock()
            .expect("images lock")
            .remove(image)
            .is_none()
        {
            return ImageDeleteResult::NotFound;
        }
        self.image_refs
            .lock()
            .expect("image refs lock")
            .retain(|_, image_id| image_id != image);
        ImageDeleteResult::Removed
    }
}
