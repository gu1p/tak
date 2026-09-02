use sha2::{Digest, Sha256};
use tak_core::v2::{RunSubmission, WorkspaceDescriptor, WorkspaceEntry, WorkspaceManifest};

use super::submission;

pub fn submission_with_spec(key: &str, secret: &str, contents: &[u8]) -> (RunSubmission, Vec<u8>) {
    let archive = archive(contents);
    let manifest = WorkspaceManifest::new([WorkspaceEntry::file(
        "TASKS.py",
        false,
        contents.len() as u64,
        &format!("{:x}", Sha256::digest(contents)),
    )
    .unwrap()])
    .unwrap();
    let mut request = submission(key, secret);
    request.run.workspace = WorkspaceDescriptor {
        manifest,
        archive_sha256: format!("{:x}", Sha256::digest(&archive)),
        archive_size: archive.len() as u64,
    };
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    (request, archive)
}

fn archive(contents: &[u8]) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    builder.mode(tar::HeaderMode::Deterministic);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(contents.len() as u64);
    header.set_cksum();
    builder
        .append_data(&mut header, "TASKS.py", contents)
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
}
