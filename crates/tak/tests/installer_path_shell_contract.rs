#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn installer_paths_are_literal_when_loading_shell_startup_files() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for installer in ["get-tak.sh", "get-takd.sh", "install-locally.sh"] {
        let source = fs::read_to_string(repo.join(installer)).unwrap();
        let definitions = source.strip_suffix("main \"$@\"\n").unwrap();
        let temp = tempfile::tempdir().unwrap();
        let rc = temp.path().join("profile");
        fs::write(&rc, "# existing profile\n").unwrap();
        let marker = temp.path().join("executed");
        let install_dir = temp.path().join(
            "a'b\"$(touch \"$TEST_MARKER\")`touch \"$TEST_MARKER\"`\\\n# existing profile\nend\n",
        );
        let setup = format!(
            "{definitions}\nactive_shell_rc() {{ printf '%s' \"$TEST_RC\"; }}\n\
             ensure_path \"$TEST_INSTALL_DIR\"\nPATH=/usr/bin:/bin\nensure_path \"$TEST_INSTALL_DIR\"\n"
        );
        let generated = Command::new("/bin/bash")
            .args(["-c", &setup])
            .env("PATH", "/usr/bin:/bin")
            .env("TEST_RC", &rc)
            .env("TEST_INSTALL_DIR", &install_dir)
            .output()
            .unwrap();
        assert!(generated.status.success(), "{installer}: {generated:?}");
        for shell in ["/bin/bash", "/bin/sh"] {
            let loaded = Command::new(shell)
                .args(["-c", ". \"$TEST_RC\"; printf '%s' \"$PATH\""])
                .env("PATH", "/usr/bin:/bin")
                .env("TEST_RC", &rc)
                .env("TEST_MARKER", &marker)
                .output()
                .unwrap();
            assert!(
                !marker.exists(),
                "{installer} startup file executed the install path"
            );
            assert!(loaded.status.success(), "{installer}, {shell}: {loaded:?}");
            assert_eq!(
                String::from_utf8(loaded.stdout).unwrap(),
                format!("{}:/usr/bin:/bin", install_dir.display()),
                "{installer}, {shell} must preserve the literal path"
            );
        }
    }
}
