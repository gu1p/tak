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

#[test]
fn coverage_script_respects_inherited_cargo_target_dir_for_prebuilt_test_binaries() -> Result<()> {
    let fixture = CoverageScriptFixture::new();
    let outer_target = tempfile::tempdir()?;
    let target_dir = outer_target.path().join("cargo-target");
    let target_dir = target_dir.display().to_string();

    let output = fixture.run_with_env(&[("CARGO_TARGET_DIR", target_dir.as_str())]);

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

#[test]
fn coverage_script_normalizes_relative_cargo_target_dir_for_prebuilt_test_binaries() -> Result<()> {
    let fixture = CoverageScriptFixture::new();

    let output = fixture.run_with_env(&[("CARGO_TARGET_DIR", "relative-cargo-target")]);

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
