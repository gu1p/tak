use std::fs;

use anyhow::Result;

use crate::support::coverage_script::CoverageScriptFixture;

#[test]
fn coverage_script_uses_llvm_cov_target_dir_for_prebuilt_test_binaries() -> Result<()> {
    let fixture = CoverageScriptFixture::new();

    let output = fixture.run();

    assert!(
        output.status.success(),
        "coverage script failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(fixture.coverage_report_path())?,
        "TN:\n",
        "coverage script should emit an lcov report"
    );
    Ok(())
}
