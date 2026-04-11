fn append_tar_entry(archive: &mut Vec<u8>, path: &str, bytes: &[u8]) -> Result<()> {
    let mut header = [0_u8; 512];
    let (prefix, name) = split_tar_path(path)?;
    write_tar_bytes(&mut header[0..100], name)?;
    write_tar_octal(&mut header[100..108], 0o644)?;
    write_tar_octal(&mut header[108..116], 0)?;
    write_tar_octal(&mut header[116..124], 0)?;
    write_tar_octal(&mut header[124..136], bytes.len() as u64)?;
    write_tar_octal(&mut header[136..148], 0)?;
    header[148..156].fill(b' ');
    header[156] = b'0';
    write_tar_bytes(&mut header[257..263], "ustar")?;
    write_tar_bytes(&mut header[263..265], "00")?;
    if !prefix.is_empty() {
        write_tar_bytes(&mut header[345..500], prefix)?;
    }

    let checksum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
    write_tar_checksum(&mut header[148..156], checksum)?;
    archive.extend_from_slice(&header);
    archive.extend_from_slice(bytes);
    archive.extend(std::iter::repeat_n(0_u8, (512 - (bytes.len() % 512)) % 512));
    Ok(())
}

fn split_tar_path(path: &str) -> Result<(&str, &str)> {
    if path.len() <= 100 {
        return Ok(("", path));
    }
    let Some(split) = path.rfind('/') else {
        bail!("container build context path exceeds tar header limit: {path}");
    };
    let prefix = &path[..split];
    let name = &path[split + 1..];
    if prefix.len() > 155 || name.is_empty() || name.len() > 100 {
        bail!("container build context path exceeds tar header limit: {path}");
    }
    Ok((prefix, name))
}

fn write_tar_bytes(field: &mut [u8], value: &str) -> Result<()> {
    if value.len() > field.len() {
        bail!("container build context header overflow for `{value}`");
    }
    field.fill(0);
    field[..value.len()].copy_from_slice(value.as_bytes());
    Ok(())
}

fn write_tar_octal(field: &mut [u8], value: u64) -> Result<()> {
    let width = field.len();
    let encoded = format!("{value:o}");
    if encoded.len() + 1 > width {
        bail!("container build context numeric field overflow");
    }
    field.fill(b'0');
    let start = width - encoded.len() - 1;
    field[start..start + encoded.len()].copy_from_slice(encoded.as_bytes());
    field[width - 1] = 0;
    Ok(())
}

fn write_tar_checksum(field: &mut [u8], value: u32) -> Result<()> {
    if field.len() != 8 {
        bail!("invalid tar checksum field width");
    }
    let encoded = format!("{value:06o}");
    field[..6].copy_from_slice(encoded.as_bytes());
    field[6] = 0;
    field[7] = b' ';
    Ok(())
}
