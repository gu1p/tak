use tak_proto::{
    TOR_INVITE_WORD_COUNT, decode_tor_invite_words, encode_tor_invite, encode_tor_invite_words,
    normalize_tor_invite_word,
};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn real_v3_tor_invite_round_trips_through_word_phrase() {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    let phrase = encode_tor_invite_words(&invite).expect("encode tor invite words");

    assert_eq!(
        phrase.split_whitespace().count(),
        TOR_INVITE_WORD_COUNT,
        "unexpected phrase length: {phrase}"
    );
    assert_eq!(
        decode_tor_invite_words(&phrase).expect("decode tor invite words"),
        invite
    );
}

#[test]
fn tor_invite_word_decoder_rejects_checksum_mismatch() {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    let phrase = encode_tor_invite_words(&invite).expect("encode tor invite words");
    let mut words = phrase.split_whitespace().collect::<Vec<_>>();
    words[TOR_INVITE_WORD_COUNT - 1] = if words[TOR_INVITE_WORD_COUNT - 1] == "a" {
        "aa"
    } else {
        "a"
    };
    let invalid = words.join(" ");

    let error = decode_tor_invite_words(&invalid).expect_err("checksum mismatch should fail");
    assert!(error.to_string().contains("checksum"));
}

#[test]
fn tor_invite_word_decoder_rejects_unknown_words() {
    let error = decode_tor_invite_words(
        "zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz zzzzzzz",
    )
    .expect_err("unknown words should fail");

    assert!(error.to_string().contains("unknown"));
}

#[test]
fn tor_invite_word_normalizer_accepts_dictionary_words_case_insensitively() {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    let phrase = encode_tor_invite_words(&invite).expect("encode tor invite words");
    let first = phrase.split_whitespace().next().expect("first word");

    assert_eq!(
        normalize_tor_invite_word(&first.to_ascii_uppercase()).expect("normalize word"),
        first
    );
    assert!(normalize_tor_invite_word("zzzzzzz").is_err());
}

#[test]
fn tor_invite_word_encoder_rejects_non_v3_onion_hosts() {
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode short tor invite");

    let error = encode_tor_invite_words(&invite).expect_err("non-v3 onion should fail");
    assert!(error.to_string().contains("v3"));
}
