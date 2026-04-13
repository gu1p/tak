fn main() {
    let version = std::env::var("TAK_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=TAK_VERSION={version}");
    println!("cargo:rerun-if-env-changed=TAK_BUILD_VERSION");
    println!("cargo:rerun-if-changed=../../README.md");
    println!("cargo:rerun-if-changed=../../examples/README.md");
    println!("cargo:rerun-if-changed=../../examples/catalog.toml");
    println!("cargo:rerun-if-changed=../tak-loader/src/loader/dsl_stubs.pyi");
}
