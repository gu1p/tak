pub fn assert_tor_secret_warning(text: &str) {
    for expected in [
        "The Tor invite/address is a secret, not just a location.",
        "Anyone with it can submit jobs and read outputs/logs.",
        "Do not paste it into shared chats, issue trackers, screenshots, or logs.",
        "Rotate the onion address if exposed.",
        "Tak remote does not provide multi-user isolation.",
    ] {
        assert!(
            text.contains(expected),
            "missing warning `{expected}`:\n{text}"
        );
    }
}
