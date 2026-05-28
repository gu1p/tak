use super::*;

pub(super) fn append_download_bytes(partial: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = partial.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create output sync directory {}",
                parent.to_string_lossy()
            )
        })?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(partial)
        .with_context(|| format!("failed to open partial output {}", partial.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("failed to write partial output {}", partial.display()))
}

pub(super) fn partial_size(path: &Path) -> Result<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(err) => Err(err).with_context(|| format!("failed to stat {}", path.display())),
    }
}

pub(super) fn remove_partial(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {}", path.display())),
    }
}

pub(super) fn partial_download_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    destination.with_file_name(format!("{name}.tak-part"))
}
