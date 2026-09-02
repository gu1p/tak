pub(super) const REQUIRED_TOKENS: [&str; 12] = [
    "current directory",
    "`TASKS.py`",
    "`tak make`",
    "`tak-make`",
    "`module_spec(spec_version=2, ...)`",
    "`takd serve`",
    "`tak runs attach RUN_ID`",
    "`tak runs outputs RUN_ID --to DIR`",
    "`Balanced`",
    "`SharedWorkspace(max_parallel_tasks=N)`",
    "20 GiB",
    "unix-socket",
];

pub(super) const REMOVED_TOKENS: [&str; 3] = [
    "recursive module discovery",
    "discovers all `TASKS.py`",
    "`tak daemon start`",
];

pub(super) const TOR_FIRST_TOKENS: [&str; 6] = [
    "direct and Tor inventory",
    "concrete placement candidates",
    "direct or Tor transport",
    "Remote worker protocol v2",
    "fencing tokens",
    "protocol fallback",
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
