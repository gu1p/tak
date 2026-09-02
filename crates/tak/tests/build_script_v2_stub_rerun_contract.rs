use std::fs;
use std::path::PathBuf;

#[test]
fn docs_build_tracks_the_shipped_v2_dsl_stub() {
    let build =
        fs::read_to_string(repo_root().join("crates/tak/build.rs")).expect("read tak build script");
    assert!(
        build.contains("../tak-loader/src/loader/dsl_stubs_v2.pyi"),
        "Tak docs build must rerun when the v2 typed DSL changes:\n{build}"
    );
    assert!(
        !build.contains("../tak-loader/src/loader/dsl_stubs.pyi"),
        "Tak docs build still tracks the removed v1 typed DSL:\n{build}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
