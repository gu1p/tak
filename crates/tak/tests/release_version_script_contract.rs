//! Contract tests for release version calculation.

use anyhow::Result;

use support::GitFixture;

#[path = "release_version_script_contract/support.rs"]
mod support;

#[test]
fn computes_first_patch_release_when_no_strict_tags_exist() -> Result<()> {
    let fixture = GitFixture::new()?;

    let output = fixture.compute("0.1.0")?;

    assert_eq!(output, "tag=v0.1.1\nversion=0.1.1\n");
    Ok(())
}

#[test]
fn increments_from_latest_strict_patch_tag() -> Result<()> {
    let fixture = GitFixture::new()?;
    fixture.tag("v0.1.1")?;
    fixture.tag("v0.1.2")?;
    fixture.commit("next")?;

    let output = fixture.compute("0.1.0")?;

    assert_eq!(output, "tag=v0.1.3\nversion=0.1.3\n");
    Ok(())
}

#[test]
fn reuses_existing_strict_tag_for_current_commit() -> Result<()> {
    let fixture = GitFixture::new()?;
    fixture.tag("v0.1.2")?;

    let output = fixture.compute("0.1.0")?;

    assert_eq!(output, "tag=v0.1.2\nversion=0.1.2\n");
    Ok(())
}

#[test]
fn ignores_legacy_sha_suffixed_tags() -> Result<()> {
    let fixture = GitFixture::new()?;
    fixture.tag("v0.1.0-deadbeef")?;

    let output = fixture.compute("0.1.0")?;

    assert_eq!(output, "tag=v0.1.1\nversion=0.1.1\n");
    Ok(())
}
