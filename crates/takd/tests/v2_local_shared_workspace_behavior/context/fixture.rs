use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

const FILES: [(&str, &[u8]); 2] = [("producer.txt", b"producer"), ("consumer.txt", b"consumer")];

pub(super) fn manifest() -> WorkspaceManifest {
    WorkspaceManifest::new(FILES.map(|(name, body)| {
        WorkspaceEntry::file(
            name,
            false,
            body.len() as u64,
            &format!("{:x}", Sha256::digest(body)),
        )
        .unwrap()
    }))
    .unwrap()
}

pub(super) fn archive() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut builder = tar::Builder::new(&mut bytes);
    for (name, body) in FILES {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(body.len() as u64);
        header.set_cksum();
        builder.append_data(&mut header, name, body).unwrap();
    }
    builder.finish().unwrap();
    drop(builder);
    bytes
}
