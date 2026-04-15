use std::io::Cursor;

use anyhow::{Context, Result};
use bollard::Docker;

use super::{PROBE_FALLBACK_IMAGE, PROBE_HELPER_BINARY, PROBE_IMAGE_AARCH64, PROBE_IMAGE_X86_64};

const PROBE_HELPER_X86_64: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/exec-root-probe/busybox-x86_64"
));
const PROBE_HELPER_AARCH64: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/exec-root-probe/busybox-aarch64"
));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeImageSource {
    Embedded { helper_bytes: &'static [u8] },
    Registry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ProbeImageSpec {
    image: &'static str,
    source: ProbeImageSource,
}

impl ProbeImageSpec {
    fn embedded(image: &'static str, helper_bytes: &'static [u8]) -> Self {
        Self {
            image,
            source: ProbeImageSource::Embedded { helper_bytes },
        }
    }

    fn registry(image: &'static str) -> Self {
        Self {
            image,
            source: ProbeImageSource::Registry,
        }
    }

    pub(super) fn image(self) -> &'static str {
        self.image
    }

    pub(super) fn helper_bytes(self) -> Option<&'static [u8]> {
        match self.source {
            ProbeImageSource::Embedded { helper_bytes } => Some(helper_bytes),
            ProbeImageSource::Registry => None,
        }
    }
}

pub(super) async fn resolve_probe_image(docker: &Docker) -> ProbeImageSpec {
    match docker.version().await {
        Ok(version) => probe_image_for_arch(version.arch.as_deref()),
        Err(err) => {
            let fallback = probe_image_for_current_target();
            tracing::warn!(
                "failed to inspect container engine architecture during exec-root probe; falling back to {}: {err:#}",
                fallback.image()
            );
            fallback
        }
    }
}

pub(super) fn build_probe_image_context(helper_bytes: &[u8]) -> Result<Vec<u8>> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    builder.mode(tar::HeaderMode::Deterministic);
    append_probe_context_entry(
        &mut builder,
        "Dockerfile",
        probe_dockerfile().as_bytes(),
        0o644,
    )?;
    append_probe_context_entry(&mut builder, "busybox", helper_bytes, 0o755)?;
    builder
        .finish()
        .context("failed to finalize exec-root probe image context")?;
    drop(builder);
    Ok(archive)
}

fn probe_image_for_arch(arch: Option<&str>) -> ProbeImageSpec {
    let normalized = arch.map(|value| value.trim().to_ascii_lowercase());
    match normalized.as_deref() {
        Some("x86_64") | Some("amd64") => {
            ProbeImageSpec::embedded(PROBE_IMAGE_X86_64, PROBE_HELPER_X86_64)
        }
        Some("aarch64") | Some("arm64") => {
            ProbeImageSpec::embedded(PROBE_IMAGE_AARCH64, PROBE_HELPER_AARCH64)
        }
        _ => ProbeImageSpec::registry(PROBE_FALLBACK_IMAGE),
    }
}

fn probe_image_for_current_target() -> ProbeImageSpec {
    match std::env::consts::ARCH {
        "x86_64" => ProbeImageSpec::embedded(PROBE_IMAGE_X86_64, PROBE_HELPER_X86_64),
        "aarch64" => ProbeImageSpec::embedded(PROBE_IMAGE_AARCH64, PROBE_HELPER_AARCH64),
        _ => ProbeImageSpec::registry(PROBE_FALLBACK_IMAGE),
    }
}

fn append_probe_context_entry(
    builder: &mut tar::Builder<&mut Vec<u8>>,
    path: &str,
    contents: &[u8],
    mode: u32,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(mode);
    header.set_uid(0);
    header.set_gid(0);
    header.set_mtime(0);
    header.set_cksum();
    builder
        .append_data(&mut header, path, Cursor::new(contents))
        .with_context(|| format!("failed to append exec-root probe context entry {path}"))?;
    Ok(())
}

fn probe_dockerfile() -> String {
    format!(
        "FROM scratch\nCOPY busybox {PROBE_HELPER_BINARY}\nENTRYPOINT [\"{PROBE_HELPER_BINARY}\"]\n"
    )
}
