use std::fs;
use std::path::PathBuf;

#[test]
fn historical_env_race_note_describes_the_v2_resolution() {
    let note = fs::read_to_string(repo_root().join("docs/test-env-parallel-race.md"))
        .expect("read environment race note");
    for required in ["Historical", "daemon-owned", "pass_env"] {
        assert!(
            note.contains(required),
            "note missing v2 resolution `{required}`"
        );
    }
    for removed in [
        "crates/tak-exec/src/client_remotes.rs",
        "crates/tak-exec/src/engine/placement_remote.rs",
        "crates/tak-exec/src/engine/public_types.rs",
        "the underlying race remains",
    ] {
        assert!(
            !note.contains(removed),
            "note recommends removed v1 path `{removed}`"
        );
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
