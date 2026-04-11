use tak_core::model::normalize_container_image_reference;

#[test]
fn container_image_reference_normalizes_digest_pinned_values() {
    let first = normalize_container_image_reference(
        " ghcr.io/acme/api@SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA ",
    )
    .expect("digest reference should normalize");
    let second = normalize_container_image_reference(
        "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .expect("equivalent digest reference should normalize");

    assert_eq!(
        first.canonical,
        "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(first.canonical, second.canonical);
    assert!(first.digest_pinned);
    assert!(second.digest_pinned);
}

#[test]
fn container_image_reference_rejects_malformed_digests() {
    for image in [
        "ghcr.io/acme/api@sha256",
        "ghcr.io/acme/api@sha256:",
        "ghcr.io/acme/api@sha256:abc",
    ] {
        let error = normalize_container_image_reference(image)
            .expect_err("malformed digest references must be rejected");
        assert!(error.to_string().contains("digest"));
    }
}

#[test]
fn container_image_reference_policy_explicitly_allows_mutable_tags() {
    let normalized =
        normalize_container_image_reference("tak/test:v1").expect("mutable tag should be allowed");
    assert_eq!(normalized.canonical, "tak/test:v1");
    assert!(!normalized.digest_pinned);
}
