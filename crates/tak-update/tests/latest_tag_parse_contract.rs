use tak_update::release_client::{ReleaseCoordinates, latest_release_url, tag_from_latest_url};

#[test]
fn parses_tag_from_redirect_url() {
    assert_eq!(
        tag_from_latest_url("https://github.com/gu1p/tak/releases/tag/v0.1.7").unwrap(),
        "v0.1.7",
    );
    assert_eq!(
        tag_from_latest_url("https://github.com/gu1p/tak/releases/tag/v0.1.7/").unwrap(),
        "v0.1.7",
    );
    assert_eq!(
        tag_from_latest_url("https://github.com/gu1p/tak/releases/tag/v0.2.0?x=1").unwrap(),
        "v0.2.0",
    );
}

#[test]
fn rejects_non_tag_urls() {
    assert!(tag_from_latest_url("https://github.com/gu1p/tak/releases/latest").is_err());
    assert!(tag_from_latest_url("https://example.com/").is_err());
}

#[test]
fn builds_asset_names_and_urls() {
    let coordinates = ReleaseCoordinates::new("gu1p/tak", "v0.1.7", "x86_64-unknown-linux-musl");
    assert_eq!(
        coordinates.archive_name(),
        "tak-v0.1.7-x86_64-unknown-linux-musl.tar.gz",
    );
    assert_eq!(
        coordinates.archive_url(),
        "https://github.com/gu1p/tak/releases/download/v0.1.7/\
tak-v0.1.7-x86_64-unknown-linux-musl.tar.gz",
    );
    assert_eq!(
        coordinates.sha256_url(),
        format!("{}.sha256", coordinates.archive_url()),
    );
    assert_eq!(
        coordinates.signature_url(),
        format!("{}.minisig", coordinates.archive_url()),
    );
    assert_eq!(
        latest_release_url("gu1p/tak"),
        "https://github.com/gu1p/tak/releases/latest",
    );
}
