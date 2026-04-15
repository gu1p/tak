use std::io;
use std::io::Cursor;

pub(super) fn tar_file_entries(body: &[u8]) -> io::Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut archive = tar::Archive::new(Cursor::new(body));
    for entry in archive.entries()? {
        let entry = entry?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        entries.push(entry.path()?.to_string_lossy().to_string());
    }
    Ok(entries)
}

pub(super) fn tar_file_modes(body: &[u8]) -> io::Result<Vec<(String, u32)>> {
    let mut entries = Vec::new();
    let mut archive = tar::Archive::new(Cursor::new(body));
    for entry in archive.entries()? {
        let entry = entry?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        entries.push((
            entry.path()?.to_string_lossy().to_string(),
            entry.header().mode()?,
        ));
    }
    Ok(entries)
}
