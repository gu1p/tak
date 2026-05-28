pub(super) const REQUIRED_TOKENS: [&str; 9] = [
    "current directory",
    "`TASKS.py`",
    "`module_spec(includes=[...])`",
    "`takd init`",
    "`takd serve`",
    "`tak remote add <token>`",
    "`tak remote status`",
    "unix socket",
    "remote v1 HTTP",
];

pub(super) const REMOVED_TOKENS: [&str; 3] = [
    "recursive module discovery",
    "discovers all `TASKS.py`",
    "`tak daemon start`",
];

pub(super) const TOR_FIRST_TOKENS: [&str; 12] = [
    "`takd peers`",
    "PeerManager",
    "`/v1/node/ping`",
    "`NodePingResponse`",
    "`PeersList`",
    "`PeersEligible`",
    "`PlaceRemote`",
    "`StreamTaskEvents`",
    "`GetTaskResult`",
    "`GetOutputRange`",
    "`remotes.toml`",
    "last-good",
];

pub(super) const TOR_FIRST_REMOVED_TOKENS: [&str; 3] = [
    "tak status remains an unsupported",
    "choose a remote node from client inventory",
    "Remote task attempts use the same client-side lease path",
];

pub(super) const TOR_CAPABILITY_TOKENS: [&str; 5] = [
    "The Tor invite/address is a secret, not just a location.",
    "Anyone with it can submit jobs and read outputs/logs.",
    "Do not paste it into shared chats, issue trackers, screenshots, or logs.",
    "Rotate the onion address if exposed.",
    "Tak remote does not provide multi-user isolation.",
];
