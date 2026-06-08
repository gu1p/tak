use tak_update::archive::{ArchiveError, extract_binaries};
use tak_update::verify::VerifiedArchive;

use crate::fixtures::make_targz;

fn verified(bytes: Vec<u8>) -> VerifiedArchive {
    VerifiedArchive::for_test(bytes)
}

#[test]
fn extracts_both_binaries_byte_exact() {
    let targz = make_targz(&[("tak", b"TAK-BYTES"), ("takd", b"TAKD-BYTES")]);
    let binaries = extract_binaries(&verified(targz)).unwrap();
    assert_eq!(binaries.tak, b"TAK-BYTES".to_vec());
    assert_eq!(binaries.takd, b"TAKD-BYTES".to_vec());
}

#[test]
fn errors_when_takd_missing() {
    let targz = make_targz(&[("tak", b"only-tak")]);
    let err = extract_binaries(&verified(targz)).unwrap_err();
    assert!(matches!(err, ArchiveError::MissingMember(name) if name == "takd"));
}

#[test]
fn rejects_non_archive_bytes() {
    assert!(extract_binaries(&verified(b"not a gzip stream".to_vec())).is_err());
}

#[test]
fn rejects_duplicate_member() {
    let targz = make_targz(&[("tak", b"a"), ("takd", b"b"), ("tak", b"c")]);
    let err = extract_binaries(&verified(targz)).unwrap_err();
    assert!(matches!(err, ArchiveError::DuplicateMember(name) if name == "tak"));
}
