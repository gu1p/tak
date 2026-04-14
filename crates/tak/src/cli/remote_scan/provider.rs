use anyhow::{Result, bail};

#[derive(Clone)]
pub(super) struct CameraDescriptor {
    pub(super) label: String,
}

#[derive(Clone)]
pub(super) struct GrayFrame {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) pixels: Vec<u8>,
}

pub(super) trait CameraCatalog {
    fn list(&self) -> Vec<CameraDescriptor>;
    fn open(&self, selected_index: usize) -> Result<Box<dyn CameraSession>>;
}

pub(super) trait CameraSession {
    fn next_frame(&mut self) -> Result<GrayFrame>;
}

pub(super) fn load_catalog() -> Result<Box<dyn CameraCatalog>> {
    if let Ok(path) = std::env::var("TAK_TEST_REMOTE_SCAN_FIXTURE") {
        return crate::cli::remote_scan::fixture::load(&path);
    }
    let catalog = crate::cli::remote_scan::linux::load()?;
    if catalog.list().is_empty() {
        bail!("no cameras available")
    }
    Ok(catalog)
}
