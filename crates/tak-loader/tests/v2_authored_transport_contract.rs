use std::fs;

use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn v2_transports_map_to_daemon_candidate_requirements() {
    for (constructor, expected) in [
        ("DirectHttps", "direct"),
        ("Any", "any"),
        ("TorOnionService", "tor"),
    ] {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = format!(
            "SPEC = module_spec(spec_version=2, tasks=[task('check', execution=Execution.Remote(transport=Transport.{constructor}()))])\nSPEC\n"
        );
        fs::write(temp.path().join("TASKS.py"), source).expect("write tasks");

        let root =
            inspect_authored_root_module(temp.path(), &LoadOptions::default()).expect("inspect v2");
        let remote = root.module.tasks[0]
            .execution
            .as_ref()
            .and_then(|execution| execution.remote())
            .expect("remote execution");
        assert_eq!(remote.transport.as_deref(), Some(expected));
    }
}
