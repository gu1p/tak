use anyhow::{Context, Result, anyhow};
use nokhwa::pixel_format::LumaFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{Camera, query};

use super::provider::{CameraCatalog, CameraDescriptor, CameraSession, GrayFrame};

pub(super) fn load() -> Result<Box<dyn CameraCatalog>> {
    let cameras = query(ApiBackend::Video4Linux).context("list linux cameras")?;
    Ok(Box::new(LinuxCatalog {
        cameras: cameras
            .into_iter()
            .map(|camera| LinuxCameraInfo {
                label: camera.human_name().to_string(),
                index: camera.index().clone(),
            })
            .collect(),
    }))
}

struct LinuxCatalog {
    cameras: Vec<LinuxCameraInfo>,
}

struct LinuxCameraInfo {
    label: String,
    index: CameraIndex,
}

impl CameraCatalog for LinuxCatalog {
    fn list(&self) -> Vec<CameraDescriptor> {
        self.cameras
            .iter()
            .map(|camera| CameraDescriptor {
                label: camera.label.clone(),
            })
            .collect()
    }

    fn open(&self, selected_index: usize) -> Result<Box<dyn CameraSession>> {
        let camera = self
            .cameras
            .get(selected_index)
            .ok_or_else(|| anyhow!("camera {selected_index} missing"))?;
        let request =
            RequestedFormat::new::<LumaFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut device =
            Camera::with_backend(camera.index.clone(), request, ApiBackend::Video4Linux)
                .context("open selected camera")?;
        device.open_stream().context("start selected camera")?;
        Ok(Box::new(LinuxSession { device }))
    }
}

struct LinuxSession {
    device: Camera,
}

impl CameraSession for LinuxSession {
    fn next_frame(&mut self) -> Result<GrayFrame> {
        let frame = self.device.frame().context("read camera frame")?;
        let image = frame
            .decode_image::<LumaFormat>()
            .context("decode camera frame")?;
        Ok(GrayFrame {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }
}
