use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod build_examples;
mod build_rust_docs;

fn main() {
    let version = env::var("TAK_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repo root should be two levels above tak crate");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    fs::create_dir_all(&out_dir)
        .unwrap_or_else(|err| panic!("failed to create {}: {err}", out_dir.display()));
    build_examples::write(repo_root, &out_dir);
    build_rust_docs::write(repo_root, &out_dir);

    print_build_triggers(repo_root);
    println!("cargo:rustc-env=TAK_VERSION={version}");
    println!("cargo:rerun-if-env-changed=TAK_BUILD_VERSION");
}

fn print_build_triggers(repo_root: &Path) {
    println!("cargo:rerun-if-changed=../../examples/catalog.toml");
    println!("cargo:rerun-if-changed=../tak-loader/src/loader/dsl_stubs.pyi");
    println!(
        "cargo:rerun-if-changed={}",
        repo_root.join("crates").display()
    );
}
