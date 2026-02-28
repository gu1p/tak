//! Optional black-box E2E for recursive remote binary build + output retrieval.

use std::fs;

use anyhow::Result;

#[allow(dead_code)]
mod support;
use support::recursive_e2e::{run_recursive_remote_task, skip_recursive_e2e_reason};

#[test]
fn e2e_remote_tor_recursive_builds_tak_binaries_and_syncs_them_back() -> Result<()> {
    if let Some(reason) = skip_recursive_e2e_reason() {
        eprintln!("skipping recursive E2E artifact contract: {reason}");
        return Ok(());
    }

    let run = run_recursive_remote_task(
        "recursive_build_artifacts",
        "set -eu; export TAK_E2E_RECURSIVE_SELF_HOSTED=0; export CARGO_TARGET_DIR=/tmp/tak-remote-build-target; export CARGO_BUILD_JOBS=1; export RUSTFLAGS=-Cdebuginfo=0; cargo build -p tak --bin tak -p takd --bin takd; mkdir -p out/bin; cp /tmp/tak-remote-build-target/debug/tak out/bin/tak; cp /tmp/tak-remote-build-target/debug/takd out/bin/takd",
    )?;

    for binary in ["tak", "takd"] {
        let path = run.workspace_root.join(format!("out/bin/{binary}"));
        let bytes = fs::read(path)?;
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[..4], b"\x7FELF");
    }

    Ok(())
}
