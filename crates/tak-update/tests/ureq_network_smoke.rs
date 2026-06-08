//! Manual smoke test against the real GitHub repo. Ignored by default (needs
//! network); run with `cargo test -p tak-update -- --ignored ureq`.

use tak_update::http::UreqReleaseClient;
use tak_update::release_client::{ReleaseClient, ReleaseCoordinates};
use tak_update::target::host_target_triple;
use tak_update::verify::{parse_sha256_line, verify_sha256};
use tak_update::version::parse_version;

#[test]
#[ignore = "hits the network (github.com)"]
fn resolves_latest_and_downloads_verified_archive() {
    let client = UreqReleaseClient::new();

    let tag = client
        .resolve_latest_tag("gu1p/tak")
        .expect("resolve latest tag");
    assert!(parse_version(&tag).is_ok(), "tag `{tag}` should parse");

    let target = host_target_triple().expect("host target triple");
    let coordinates = ReleaseCoordinates::new("gu1p/tak", &tag, &target);

    let sha_line = client
        .download_sha256(&coordinates)
        .expect("download .sha256");
    assert!(
        parse_sha256_line(&sha_line).is_ok(),
        "sha line: {sha_line:?}"
    );

    // The large archive download depends on GitHub's asset CDN, which transiently
    // 504s; tolerate that here (the daemon loop retries) and only assert the
    // verify path when the bytes actually arrive.
    match client.download_archive(&coordinates) {
        Ok(archive) => {
            assert!(archive.len() > 1024, "archive should be non-trivial");
            verify_sha256(&archive, &sha_line).expect("downloaded archive matches its checksum");
        }
        Err(err) => eprintln!("skipping archive verification (transient download error: {err})"),
    }
}
