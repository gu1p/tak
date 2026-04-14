#[cfg(target_os = "linux")]
mod app;
#[cfg(target_os = "linux")]
mod decode;
#[cfg(target_os = "linux")]
mod fixture;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod provider;
#[cfg(target_os = "linux")]
mod render;
#[cfg(target_os = "linux")]
mod scripted;
#[cfg(target_os = "linux")]
mod terminal;
#[cfg(all(test, target_os = "linux"))]
mod terminal_tests;

#[cfg(target_os = "linux")]
use anyhow::Result;
#[cfg(target_os = "linux")]
use anyhow::bail;

#[cfg(target_os = "linux")]
pub(super) async fn run_remote_scan() -> Result<()> {
    let catalog = provider::load_catalog()?;
    let cameras = catalog.list();
    if cameras.is_empty() {
        bail!("no cameras available");
    }
    let app = app::ScanApp::new(cameras);
    if let Ok(script) = std::env::var("TAK_TEST_REMOTE_SCAN_SCRIPT") {
        scripted::run(app, &*catalog, &script).await
    } else {
        terminal::run(app, &*catalog).await
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) async fn run_remote_scan() -> anyhow::Result<()> {
    anyhow::bail!("remote scan is currently supported only on Linux")
}
