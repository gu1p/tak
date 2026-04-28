use super::FakeDockerDaemonState;

#[path = "image/delete.rs"]
mod delete;

pub(in crate::support::fake_docker_daemon) use delete::ImageDeleteResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::support::fake_docker_daemon) struct ImageInfo {
    pub(in crate::support::fake_docker_daemon) id: String,
    pub(in crate::support::fake_docker_daemon) size: u64,
}

impl FakeDockerDaemonState {
    pub(in crate::support::fake_docker_daemon) fn image_present(&self, image: &str) -> bool {
        self.image_info(image).is_some()
    }

    pub(in crate::support::fake_docker_daemon) fn image_info(
        &self,
        image: &str,
    ) -> Option<ImageInfo> {
        if let Some(id) = self
            .image_refs
            .lock()
            .expect("image refs lock")
            .get(image)
            .cloned()
        {
            let size = self.images.lock().expect("images lock").get(&id).copied()?;
            return Some(ImageInfo { id, size });
        }
        self.images
            .lock()
            .expect("images lock")
            .get(image)
            .copied()
            .map(|size| ImageInfo {
                id: image.to_string(),
                size,
            })
    }

    pub(in crate::support::fake_docker_daemon) fn set_image(
        &self,
        image_ref: &str,
        image_id: &str,
        size: u64,
    ) {
        self.images
            .lock()
            .expect("images lock")
            .insert(image_id.to_string(), size);
        self.image_refs
            .lock()
            .expect("image refs lock")
            .insert(image_ref.to_string(), image_id.to_string());
    }

    pub(in crate::support::fake_docker_daemon) fn remove_image(&self, image: &str) {
        let removed_id = self
            .image_refs
            .lock()
            .expect("image refs lock")
            .remove(image);
        if let Some(id) = removed_id {
            let still_referenced = self
                .image_refs
                .lock()
                .expect("image refs lock")
                .values()
                .any(|candidate| candidate == &id);
            if !still_referenced {
                self.images.lock().expect("images lock").remove(&id);
            }
            return;
        }
        self.images.lock().expect("images lock").remove(image);
    }

    pub(in crate::support::fake_docker_daemon) fn fail_image_removal(&self, status_code: u16) {
        *self
            .image_removal_failure_status
            .lock()
            .expect("image removal failure status lock") = Some(status_code);
    }
}
