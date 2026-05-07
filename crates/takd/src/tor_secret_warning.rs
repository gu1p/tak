pub(crate) const LINES: [&str; 5] = [
    "The Tor invite/address is a secret, not just a location.",
    "Anyone with it can submit jobs and read outputs/logs.",
    "Do not paste it into shared chats, issue trackers, screenshots, or logs.",
    "Rotate the onion address if exposed.",
    "Tak remote does not provide multi-user isolation.",
];

pub(crate) fn text() -> String {
    LINES.join("\n")
}

pub(crate) fn stderr_text() -> String {
    format!("WARNING: {}\n", text())
}
