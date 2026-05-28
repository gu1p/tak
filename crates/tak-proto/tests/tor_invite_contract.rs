use tak_proto::{
    decode_tor_invite, decode_tor_invite_payload, encode_tor_invite, encode_tor_invite_with_bearer,
};

#[test]
fn tor_invites_round_trip_base_url_with_checksum() {
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode tor invite");
    assert!(
        invite.starts_with("takd:tor:builder-a.onion:"),
        "unexpected invite: {invite}"
    );
    assert_eq!(
        decode_tor_invite(&invite).expect("decode tor invite"),
        "http://builder-a.onion"
    );
}

#[test]
fn tor_invites_can_carry_bearer_token_for_authenticated_peers() {
    let invite = encode_tor_invite_with_bearer("http://builder-a.onion", "secret-token")
        .expect("encode tor invite");
    let payload = decode_tor_invite_payload(&invite).expect("decode tor invite payload");

    assert_eq!(payload.base_url, "http://builder-a.onion");
    assert_eq!(payload.bearer_token, "secret-token");
    assert_eq!(
        decode_tor_invite(&invite).expect("decode tor invite"),
        "http://builder-a.onion"
    );
}

#[test]
fn legacy_tor_invites_decode_with_empty_bearer_token() {
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode tor invite");
    let payload = decode_tor_invite_payload(&invite).expect("decode tor invite payload");

    assert_eq!(payload.base_url, "http://builder-a.onion");
    assert_eq!(payload.bearer_token, "");
}

#[test]
fn tor_invites_reject_checksum_mismatches() {
    let error = decode_tor_invite("takd:tor:builder-a.onion:00000")
        .expect_err("invite with bad checksum should fail");
    assert!(
        error.to_string().contains("checksum"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn tor_invites_reject_non_onion_hosts() {
    let error = encode_tor_invite("http://example.com").expect_err("non-onion host should fail");
    assert!(
        error.to_string().contains(".onion"),
        "unexpected error: {error:#}"
    );
}
