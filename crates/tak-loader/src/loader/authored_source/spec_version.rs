use std::path::Path;

use anyhow::Result;
use ruff_text_size::TextRange;

use super::{ModuleDeclaration, SpecVersionMarker, authored_error};

const MIGRATION_SUMMARY: &str = "Migration summary for the coordinated v2 release: after upgrading, run `tak docs dump`; add literal `spec_version=2` to every TASKS.py module; all execution is daemon-owned by local `takd`; `RemoteSelection.Balanced()` replaces `Shuffle()`; `Workspace()` and `Paths(...)` are isolated snapshots; use `SharedWorkspace(max_parallel_tasks=N)` for shared undeclared writes; declare outputs and `pass_env`.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoredSpecVersion {
    LegacyBootstrap,
    V2,
}

#[derive(Clone, Copy)]
pub(crate) struct LegacyBootstrapAdmission {
    marker_range: Option<TextRange>,
    authored_description: &'static str,
}

pub(crate) fn admit_legacy_bootstrap(
    path: &Path,
    source: &str,
    declaration: Option<&ModuleDeclaration>,
) -> Result<LegacyBootstrapAdmission> {
    let Some(declaration) = declaration else {
        return Ok(LegacyBootstrapAdmission {
            marker_range: None,
            authored_description: "authored source had no direct module_spec declaration",
        });
    };
    match declaration.version {
        SpecVersionMarker::Omitted => Ok(LegacyBootstrapAdmission {
            marker_range: Some(declaration.marker_range),
            authored_description: "authored version was omitted",
        }),
        SpecVersionMarker::Literal(1) => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            format!(
                "explicit module_spec(spec_version=1) is rejected. {MIGRATION_SUMMARY} This build recognizes but does not load v2 modules yet; omission remains a temporary repository bootstrap."
            ),
        )),
        SpecVersionMarker::Literal(2) => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            "module_spec(spec_version=2) cannot enter legacy TASKS.py loading; no legacy adapter was attempted",
        )),
        SpecVersionMarker::Literal(version) => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            format!(
                "unsupported spec_version={version}; version 2 is the migration target, but this build does not load it yet. Upgrade tak, takd, and workers together when the v2 release is available"
            ),
        )),
    }
}

pub(crate) fn classify_authored_version(
    path: &Path,
    source: &str,
    declaration: Option<&ModuleDeclaration>,
) -> Result<AuthoredSpecVersion> {
    let Some(declaration) = declaration else {
        return Ok(AuthoredSpecVersion::LegacyBootstrap);
    };
    match declaration.version {
        SpecVersionMarker::Omitted => Ok(AuthoredSpecVersion::LegacyBootstrap),
        SpecVersionMarker::Literal(1) => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            format!("explicit module_spec(spec_version=1) is rejected. {MIGRATION_SUMMARY}"),
        )),
        SpecVersionMarker::Literal(2) => Ok(AuthoredSpecVersion::V2),
        SpecVersionMarker::Literal(version) => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            format!(
                "unsupported spec_version={version}; protocol v2 is required. Upgrade tak, takd, and workers together"
            ),
        )),
    }
}

pub(crate) fn validate_evaluated_version(
    path: &Path,
    source: &str,
    admission: LegacyBootstrapAdmission,
    evaluated_version: u32,
) -> Result<()> {
    if evaluated_version == 1 {
        return Ok(());
    }
    let message = format!(
        "evaluated spec_version={evaluated_version}; {}; legacy bootstrap requires version 1",
        admission.authored_description
    );
    match admission.marker_range {
        Some(range) => Err(authored_error(path, source, range, message)),
        None => Err(anyhow::anyhow!("{}: {message}", path.display())),
    }
}
