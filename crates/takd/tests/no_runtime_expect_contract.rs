use std::path::Path;

#[test]
fn runtime_sources_do_not_use_expect_for_recoverable_failures() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let contracts = [
        (
            "src/daemon/lease/manager_public_methods.rs",
            "expect(\"failed to persist active lease\")",
        ),
        (
            "src/daemon/lease/manager_public_methods.rs",
            "expect(\"failed to append acquire history\")",
        ),
        (
            "src/daemon/lease/mod.rs",
            "expect(\"system clock is before UNIX_EPOCH\")",
        ),
        (
            "src/daemon/remote/query_helpers.rs",
            "expect(\"system clock is before UNIX_EPOCH\")",
        ),
        (
            "src/daemon/remote/submit_store/commands.rs",
            "attempt.expect(\"validated by build_submit_idempotency_key\")",
        ),
    ];

    for (relative, denied_snippet) in contracts {
        let source = std::fs::read_to_string(crate_root.join(relative))
            .unwrap_or_else(|err| panic!("read {relative}: {err}"));
        assert!(
            !source.contains(denied_snippet),
            "runtime source {} still contains forbidden expect snippet: {}",
            relative,
            denied_snippet
        );
    }
}
