fn tar_builder(archive: &mut Vec<u8>) -> tar::Builder<&mut Vec<u8>> {
    let mut builder = tar::Builder::new(archive);
    builder.mode(tar::HeaderMode::Deterministic);
    builder
}

fn append_tar_entry(
    builder: &mut tar::Builder<&mut Vec<u8>>,
    path: &str,
    absolute_path: &Path,
    mode: u32,
) -> Result<()> {
    let mut file = std::fs::File::open(absolute_path)
        .with_context(|| format!("failed to open build context entry {path}"))?;
    let metadata = file
        .metadata()
        .with_context(|| format!("failed to read build context metadata for {path}"))?;
    let mut header = tar::Header::new_gnu();
    header.set_metadata_in_mode(&metadata, tar::HeaderMode::Deterministic);
    header.set_mode(mode);
    builder
        .append_data(&mut header, Path::new(path), &mut file)
        .with_context(|| format!("failed to append build context entry {path}"))?;
    Ok(())
}
