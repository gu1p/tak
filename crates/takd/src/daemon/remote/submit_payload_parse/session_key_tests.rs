use super::*;
use tak_proto::{ExecutionSession, OutputSelector, output_selector};

#[test]
fn reserved_storage_components_are_rejected_for_every_session_mode() {
    for key in [".", "..", " . ", " .. "] {
        for reuse in ["share_workspace", "share_paths", "container"] {
            let error = parse_remote_worker_session(&session(key, reuse))
                .expect_err("reserved storage component");

            assert!(
                format!("{error:#}").contains("reserved session.key"),
                "unexpected error for key {key:?} and reuse {reuse:?}: {error:#}"
            );
        }
    }
}

#[test]
fn ordinary_and_leading_dot_session_keys_remain_valid() {
    for key in ["run-uuid-rust", ".cache"] {
        let parsed = parse_remote_worker_session(&session(key, "share_workspace"))
            .expect("valid session key");

        assert_eq!(parsed.key, key);
    }
}

fn session(key: &str, reuse: &str) -> ExecutionSession {
    ExecutionSession {
        key: key.into(),
        name: "test".into(),
        reuse: reuse.into(),
        share_paths: vec![OutputSelector {
            kind: Some(output_selector::Kind::Path("cache".into())),
        }],
    }
}
