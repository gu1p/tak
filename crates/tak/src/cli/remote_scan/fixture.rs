use anyhow::{Context, Result, anyhow};
use qrcode::{Color, QrCode};
use serde::Deserialize;
use std::fs;

use super::provider::{CameraCatalog, CameraDescriptor, CameraSession, GrayFrame};

pub(super) fn load(path: &str) -> Result<Box<dyn CameraCatalog>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path))?;
    let fixture: ScanFixture = toml::from_str(&raw).with_context(|| format!("decode {}", path))?;
    Ok(Box::new(FixtureCatalog::new(fixture)))
}

#[derive(Deserialize)]
struct ScanFixture {
    cameras: Vec<FixtureCamera>,
}

#[derive(Clone, Deserialize)]
struct FixtureCamera {
    #[allow(dead_code)]
    id: String,
    name: String,
    frames: Vec<FixtureFrame>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FixtureFrame {
    Blank { width: u32, height: u32 },
    QrPayload { payload: String, width: u32 },
}

struct FixtureCatalog {
    cameras: Vec<FixtureCamera>,
}

impl FixtureCatalog {
    fn new(fixture: ScanFixture) -> Self {
        Self {
            cameras: fixture.cameras,
        }
    }
}

impl CameraCatalog for FixtureCatalog {
    fn list(&self) -> Vec<CameraDescriptor> {
        self.cameras
            .iter()
            .map(|camera| CameraDescriptor {
                label: camera.name.clone(),
            })
            .collect()
    }

    fn open(&self, selected_index: usize) -> Result<Box<dyn CameraSession>> {
        let camera = self
            .cameras
            .get(selected_index)
            .ok_or_else(|| anyhow!("fixture camera {selected_index} missing"))?
            .clone();
        Ok(Box::new(FixtureSession { camera, next: 0 }))
    }
}

struct FixtureSession {
    camera: FixtureCamera,
    next: usize,
}

impl CameraSession for FixtureSession {
    fn next_frame(&mut self) -> Result<GrayFrame> {
        let frame = self.camera.frames[self.next % self.camera.frames.len()].clone();
        self.next = self.next.saturating_add(1);
        match frame {
            FixtureFrame::Blank { width, height } => Ok(GrayFrame {
                width,
                height,
                pixels: vec![255; width as usize * height as usize],
            }),
            FixtureFrame::QrPayload { payload, width } => qr_payload_frame(&payload, width),
        }
    }
}

fn qr_payload_frame(payload: &str, width: u32) -> Result<GrayFrame> {
    let code = QrCode::new(payload.as_bytes())?;
    let quiet = 4_u32;
    let modules = code.width() as u32;
    let pitch = (width / (modules + quiet * 2)).max(1);
    let image_width = (modules + quiet * 2) * pitch;
    let mut pixels = vec![255_u8; image_width as usize * image_width as usize];
    for y in 0..modules {
        for x in 0..modules {
            if code[(x as usize, y as usize)] != Color::Dark {
                continue;
            }
            fill_square(
                &mut pixels,
                image_width as usize,
                x + quiet,
                y + quiet,
                pitch,
            );
        }
    }
    Ok(GrayFrame {
        width: image_width,
        height: image_width,
        pixels,
    })
}

fn fill_square(pixels: &mut [u8], stride: usize, x: u32, y: u32, pitch: u32) {
    let start_x = x as usize * pitch as usize;
    let start_y = y as usize * pitch as usize;
    for row in start_y..(start_y + pitch as usize) {
        for col in start_x..(start_x + pitch as usize) {
            pixels[row * stride + col] = 0;
        }
    }
}
