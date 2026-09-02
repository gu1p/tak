use crate::support::unix_socket_path::short_socket_bind_path;

#[test]
fn checkout_local_absolute_socket_gets_a_short_equivalent_bind_path() {
    let current = std::env::current_dir().expect("current directory");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(current, manifest);
    let checkout = manifest.ancestors().nth(2).expect("workspace root");
    let absolute = checkout.join(".tmp/test-tmp/docker.sock");
    assert!(absolute.as_os_str().len() > 103);

    let bind_path = short_socket_bind_path(&absolute);

    assert!(bind_path.is_relative());
    assert_eq!(
        bind_path,
        std::path::Path::new("../../.tmp/test-tmp/docker.sock")
    );
    assert!(bind_path.as_os_str().len() <= 103);
}

#[test]
fn socket_outside_current_directory_keeps_its_absolute_path() {
    let absolute = std::path::Path::new("/outside-checkout/docker.sock");

    assert_eq!(short_socket_bind_path(absolute), absolute);
}
