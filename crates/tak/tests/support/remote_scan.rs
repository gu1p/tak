#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::{Command as StdCommand, Output, Stdio};

use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct CameraFixture<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub frames: &'a [FrameFixture<'a>],
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FrameFixture<'a> {
    Blank { width: u32, height: u32 },
    QrPayload { payload: &'a str, width: u32 },
}

#[derive(Serialize)]
struct ScanFixture<'a> {
    cameras: &'a [CameraFixture<'a>],
}

pub fn write_scan_fixture(path: &Path, cameras: &[CameraFixture<'_>]) -> Result<()> {
    fs::write(path, toml::to_string(&ScanFixture { cameras })?)?;
    Ok(())
}

pub fn write_single_camera_qr_fixture(path: &Path, payload: &str) -> Result<()> {
    let frames = [
        FrameFixture::Blank {
            width: 192,
            height: 192,
        },
        FrameFixture::QrPayload {
            payload,
            width: 192,
        },
    ];
    let cameras = [CameraFixture {
        id: "cam0",
        name: "Desk Camera",
        frames: &frames,
    }];
    write_scan_fixture(path, &cameras)
}

pub fn run_scan(config_root: &Path, fixture_path: &Path, script: &str) -> Result<Output> {
    run_scan_with_env(config_root, fixture_path, script, &[])
}

pub fn run_scan_with_env(
    config_root: &Path,
    fixture_path: &Path,
    script: &str,
    envs: &[(&str, String)],
) -> Result<Output> {
    let mut command = StdCommand::new(super::tak_bin());
    command
        .args(["remote", "scan"])
        .env("XDG_CONFIG_HOME", config_root)
        .env("TAK_TEST_REMOTE_SCAN_FIXTURE", fixture_path)
        .env("TAK_TEST_REMOTE_SCAN_SCRIPT", script)
        .stdin(Stdio::null());
    for (key, value) in envs {
        command.env(key, value);
    }
    Ok(command.output()?)
}
