#![allow(dead_code)]

use std::collections::BTreeMap;
use std::process::Command as StdCommand;

const LIVE_TOR_PROBE_TIMEOUT_MS: &str = "300000";
const LIVE_TOR_PROBE_BACKOFF_MS: &str = "1000";

pub fn insert_live_tor_probe_env(env: &mut BTreeMap<String, String>) {
    env.insert(
        "TAK_TOR_PROBE_TIMEOUT_MS".to_string(),
        LIVE_TOR_PROBE_TIMEOUT_MS.to_string(),
    );
    env.insert(
        "TAK_TOR_PROBE_BACKOFF_MS".to_string(),
        LIVE_TOR_PROBE_BACKOFF_MS.to_string(),
    );
}

pub fn apply_live_tor_probe_env(command: &mut StdCommand) {
    command
        .env("TAK_TOR_PROBE_TIMEOUT_MS", LIVE_TOR_PROBE_TIMEOUT_MS)
        .env("TAK_TOR_PROBE_BACKOFF_MS", LIVE_TOR_PROBE_BACKOFF_MS);
}
