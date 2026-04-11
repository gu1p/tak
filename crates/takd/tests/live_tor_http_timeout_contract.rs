mod support;

use std::time::Duration;

use support::env::env_lock;
use support::live_tor_http::live_tor_test_timeout;

#[test]
fn live_tor_http_timeout_defaults_to_three_minutes() {
    let _lock = env_lock();
    unsafe {
        std::env::remove_var("TAK_LIVE_TOR_TEST_TIMEOUT_SECS");
    }

    assert_eq!(live_tor_test_timeout(), Duration::from_secs(180));
}

#[test]
fn live_tor_http_timeout_uses_env_override() {
    let _lock = env_lock();
    unsafe {
        std::env::set_var("TAK_LIVE_TOR_TEST_TIMEOUT_SECS", "420");
    }

    assert_eq!(live_tor_test_timeout(), Duration::from_secs(420));

    unsafe {
        std::env::remove_var("TAK_LIVE_TOR_TEST_TIMEOUT_SECS");
    }
}
