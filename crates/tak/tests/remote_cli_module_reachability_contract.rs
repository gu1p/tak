use std::fs;
use std::path::Path;

#[test]
fn every_remote_cli_contract_is_registered_and_removed_docs_helpers_stay_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registration = fs::read_to_string(root.join("tests/remote_cli_contracts.rs")).unwrap();
    for entry in fs::read_dir(root.join("tests")).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if entry.file_type().unwrap().is_file()
            && name.starts_with("remote_cli_")
            && name.ends_with(".rs")
            && name != "remote_cli_contracts.rs"
            && name != "remote_cli_module_reachability_contract.rs"
        {
            assert!(
                registration.contains(&format!("#[path = \"{name}\"]")),
                "unregistered remote CLI contract remains: {name}"
            );
        }
    }
    assert!(!root.join("src/docs/markdown.rs").exists());
}
