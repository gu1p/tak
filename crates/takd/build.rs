use std::env;

fn main() {
    let version = env::var("TAK_BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=TAKD_VERSION={version}");
    println!("cargo:rerun-if-env-changed=TAK_BUILD_VERSION");
}
