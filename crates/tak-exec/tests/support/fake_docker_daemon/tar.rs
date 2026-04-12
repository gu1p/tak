use std::io;

pub(super) fn tar_file_entries(body: &[u8]) -> io::Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut index = 0;
    while index + 512 <= body.len() {
        let header = &body[index..index + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }

        let name = tar_path(header);
        let size = tar_size(header)?;
        if !name.is_empty() && header[156] != b'5' {
            entries.push(name);
        }
        index += 512 + size.div_ceil(512) * 512;
    }
    Ok(entries)
}

pub(super) fn tar_file_modes(body: &[u8]) -> io::Result<Vec<(String, u32)>> {
    let mut entries = Vec::new();
    let mut index = 0;
    while index + 512 <= body.len() {
        let header = &body[index..index + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }

        let name = tar_path(header);
        let size = tar_size(header)?;
        if !name.is_empty() && header[156] != b'5' {
            entries.push((name, tar_mode(header)?));
        }
        index += 512 + size.div_ceil(512) * 512;
    }
    Ok(entries)
}

fn tar_path(header: &[u8]) -> String {
    let name = trim_tar_field(&header[0..100]);
    let prefix = trim_tar_field(&header[345..500]);
    if prefix.is_empty() {
        name
    } else {
        format!("{prefix}/{name}")
    }
}

fn trim_tar_field(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).to_string()
}

fn tar_size(header: &[u8]) -> io::Result<usize> {
    let raw = trim_tar_field(&header[124..136]);
    if raw.trim().is_empty() {
        return Ok(0);
    }
    usize::from_str_radix(raw.trim(), 8)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

fn tar_mode(header: &[u8]) -> io::Result<u32> {
    let raw = trim_tar_field(&header[100..108]);
    if raw.trim().is_empty() {
        return Ok(0);
    }
    u32::from_str_radix(raw.trim(), 8)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}
