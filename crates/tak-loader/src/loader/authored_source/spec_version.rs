use std::path::Path;

use super::{ModuleDeclaration, SpecVersionMarker, authored_error};
use anyhow::Result;

const MIGRATION_SUMMARY: &str = "Migration summary for the coordinated v2 release: after upgrading, run `tak docs dump`; add literal `spec_version=2` to every TASKS.py module; all execution is daemon-owned by local `takd`; `RemoteSelection.Balanced()` replaces `Shuffle()`; `max_pending` was removed from `queue_def`, so use queue `slots`; Container `command` was removed, so use task steps; container mount sources must be workspace-relative; `Workspace()` and `Paths(...)` are isolated snapshots; use `SharedWorkspace(max_parallel_tasks=N)` for shared undeclared writes; declare outputs and `pass_env`.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuthoredSpecVersion {
    V2,
}

pub(crate) fn classify_authored_version(
    path: &Path,
    source: &str,
    declaration: Option<&ModuleDeclaration>,
) -> Result<AuthoredSpecVersion> {
    let Some(declaration) = declaration else {
        return Err(anyhow::anyhow!(
            "{}: explicit module_spec(..., spec_version=2) is required. {MIGRATION_SUMMARY}",
            path.display()
        ));
    };
    match declaration.version {
        SpecVersionMarker::Omitted => Err(authored_error(
            path,
            source,
            declaration.marker_range,
            format!(
                "module_spec omitted spec_version; explicit spec_version=2 is required. {MIGRATION_SUMMARY}"
            ),
        )),
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
