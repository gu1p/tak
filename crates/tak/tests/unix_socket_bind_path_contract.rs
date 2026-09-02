use crate::support::unix_socket_bind_path::short_bind_path;

#[test]
fn long_checkout_local_socket_uses_short_equivalent_bind_path() {
    let current = std::env::current_dir().expect("current test directory");
    let mut requested = current.join(".tmp/socket-bind-contract/takd.sock");
    while requested.as_os_str().len() <= 103 {
        requested = requested
            .parent()
            .expect("socket parent")
            .join("nested/takd.sock");
    }

    let bind_path = short_bind_path(&requested);

    assert!(!bind_path.is_absolute(), "{bind_path:?}");
    assert!(bind_path.as_os_str().len() <= 103, "{bind_path:?}");
    assert_eq!(current.join(&bind_path), requested);
}

#[test]
fn socket_in_checkout_sibling_of_package_uses_relative_bind_path() {
    let current = std::env::current_dir().expect("current test directory");
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root");
    let temp = tempfile::tempdir_in(checkout.join(".tmp")).expect("checkout tempdir");
    let mut parent = temp.path().join("nested");
    while parent.join("takd.sock").as_os_str().len() <= 103 {
        parent.push("nested");
    }
    std::fs::create_dir_all(&parent).expect("socket parent");
    let requested = parent.join("takd.sock");

    let bind_path = short_bind_path(&requested);

    assert!(!bind_path.is_absolute(), "{bind_path:?}");
    assert!(bind_path.as_os_str().len() <= 103, "{bind_path:?}");
    assert_eq!(
        current
            .join(&bind_path)
            .parent()
            .expect("resolved parent")
            .canonicalize()
            .expect("canonical resolved parent"),
        requested
            .parent()
            .expect("requested parent")
            .canonicalize()
            .expect("canonical requested parent")
    );
}
