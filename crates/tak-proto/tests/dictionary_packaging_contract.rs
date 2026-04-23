use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn tor_invite_dictionary_is_packaged_inside_tak_proto() {
    let dictionary_path = crate_root().join("src/tor_invite_words/words.txt");
    assert!(
        dictionary_path.is_file(),
        "expected bundled dictionary at {}",
        dictionary_path.display()
    );

    let source = fs::read_to_string(crate_root().join("src/tor_invite_words/dictionary.rs"))
        .expect("read dictionary source");
    assert!(
        source.contains("include_str!(\"words.txt\")"),
        "expected dictionary.rs to include the crate-local words.txt asset"
    );
}
