use std::fs;
use std::os::unix::net::UnixListener;
use std::path::Path;

use tak_core::v2::TaskContext;

use super::workspace;

#[test]
fn union_workspace_readds_explicit_gitignored_context_paths() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::create_dir_all(temp.path().join("target/reinclude")).unwrap();
    fs::create_dir_all(temp.path().join("target/private")).unwrap();
    fs::write(temp.path().join(".gitignore"), "target/\n").unwrap();
    fs::write(temp.path().join("target/reinclude/keep.txt"), "keep").unwrap();
    fs::write(temp.path().join("target/private/drop.txt"), "drop").unwrap();
    let socket = temp.path().join("target/private/agent-control.sock");
    let _socket = bind_socket(&socket);
    let context = TaskContext {
        roots: vec![".".into()],
        ignored_paths: vec![],
        use_gitignore: true,
        include: vec!["target/reinclude".into()],
    };

    let bundle = workspace::build_for_contexts(temp.path(), &[&context]).unwrap();
    let paths = bundle
        .descriptor
        .manifest
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<Vec<_>>();
    assert!(paths.contains(&"target/reinclude/keep.txt"));
    assert!(!paths.contains(&"target/private/drop.txt"));
    assert!(!paths.contains(&"target/private/agent-control.sock"));
}

#[test]
fn union_workspace_rejects_an_explicitly_included_special_entry() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::create_dir_all(temp.path().join("ignored")).unwrap();
    fs::write(temp.path().join(".gitignore"), "ignored/\n").unwrap();
    let socket = temp.path().join("ignored/agent-control.sock");
    let _socket = bind_socket(&socket);
    let context = TaskContext {
        roots: vec![".".into()],
        ignored_paths: vec![],
        use_gitignore: true,
        include: vec!["ignored/agent-control.sock".into()],
    };

    let error = workspace::build_for_contexts(temp.path(), &[&context])
        .err()
        .expect("an explicitly included socket must be rejected");
    assert!(
        error
            .to_string()
            .contains("unsupported workspace entry: ignored/agent-control.sock"),
        "{error:#}"
    );
}

fn bind_socket(path: &Path) -> UnixListener {
    let current = std::env::current_dir().unwrap();
    let bind_path = path.strip_prefix(&current).unwrap_or(path);
    UnixListener::bind(bind_path).unwrap()
}
