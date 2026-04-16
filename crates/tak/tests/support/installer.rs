#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Output};

pub fn run_installer(
    systemctl: &str,
    extra_env: &[(&str, &str)],
) -> (tempfile::TempDir, PathBuf, Output) {
    let temp = tempfile::tempdir().expect("tempdir");
    let home = temp.path().join("home");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&bin).expect("create bin");
    fs::copy(
        repo_root().join("get-takd.sh"),
        temp.path().join("get-takd.sh"),
    )
    .expect("copy");
    fs::write(bin.join("uname"), fake_uname()).expect("write uname");
    fs::write(bin.join("curl"), fake_curl()).expect("write curl");
    fs::write(bin.join("systemctl"), systemctl).expect("write systemctl");
    chmod_exec(&bin.join("uname"));
    chmod_exec(&bin.join("curl"));
    chmod_exec(&bin.join("systemctl"));

    let mut command = StdCommand::new("/bin/bash");
    command
        .arg(temp.path().join("get-takd.sh"))
        .env("HOME", &home)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_STATE_HOME", home.join(".local/state"));
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().expect("run installer");
    (temp, home, output)
}

pub fn fake_systemctl() -> &'static str {
    "#!/usr/bin/env bash\nexit 0\n"
}

pub fn failing_systemctl() -> &'static str {
    "#!/usr/bin/env bash\nprintf 'systemd unavailable\\n' >&2\nexit 1\n"
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn chmod_exec(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

fn fake_uname() -> &'static str {
    "#!/usr/bin/env bash\nif [[ \"$1\" == \"-m\" ]]; then echo x86_64; else echo Linux; fi\n"
}

fn fake_curl() -> &'static str {
    "#!/usr/bin/env bash\nif [[ \"$*\" == *\"releases/latest\"* ]]; then echo '{\"tag_name\":\"v1.0.0\"}'; exit 0; fi\nout=''; prev=''; for arg in \"$@\"; do if [[ \"$prev\" == '-o' ]]; then out=\"$arg\"; fi; prev=\"$arg\"; done\nmkdir -p \"$(dirname \"$out\")/pkg\"; cat >\"$(dirname \"$out\")/pkg/takd\" <<'EOF'\n#!/usr/bin/env bash\nset -euo pipefail\nstate_root=\"${XDG_STATE_HOME:-$HOME/.local/state}/takd\"\nconfig_root=\"${XDG_CONFIG_HOME:-$HOME/.config}/takd\"\nmkdir -p \"$state_root\" \"$config_root\"\ncase \"$1 ${2:-}\" in\n  'init '*) : ;;\n  'token show'*)\n    if [[ \"$*\" == *\"--qr\"* ]]; then\n      printf \"Scan this QR code\\n████████\\n██    ██\\n██ ██ ██\\n████████\\ntak remote add 'takd:tor:test-hidden-service.onion:ABCDE'\\ntakd:tor:test-hidden-service.onion:ABCDE\\n\"\n    else\n      echo 'takd:tor:test-hidden-service.onion:ABCDE'\n    fi\n    ;;\n  'status '*) echo 'base_url: http://test-hidden-service.onion' ;;\n  'serve '*) : ;;\nesac\nEOF\nchmod +x \"$(dirname \"$out\")/pkg/takd\"\ntar -czf \"$out\" -C \"$(dirname \"$out\")/pkg\" takd\n"
}
