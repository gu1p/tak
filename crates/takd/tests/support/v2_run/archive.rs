use std::sync::LazyLock;

pub static ARCHIVE: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    builder.mode(tar::HeaderMode::Deterministic);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(4);
    header.set_cksum();
    builder
        .append_data(&mut header, "TASKS.py", &b"spec"[..])
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
});
