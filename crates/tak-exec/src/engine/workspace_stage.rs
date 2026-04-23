/// Creates a staged workspace for remote execution from the task's normalized `CurrentState`.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};
use base64::Engine;
use tak_core::model::{ResolvedTask, build_current_state_manifest};
use zip::write::SimpleFileOptions;

use super::{RemoteWorkspaceStage, TaskOutputObserver, TaskStatusPhase};

use super::output_observer::emit_task_status_message;
use super::workspace_collect::{collect_workspace_files, materialize_manifest_files};
use super::workspace_sync::normalize_filesystem_relative_path;

pub(crate) fn stage_remote_workspace(
    task: &ResolvedTask,
    workspace_root: &Path,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<RemoteWorkspaceStage> {
    emit_task_status_message(
        output_observer,
        &task.label,
        1,
        TaskStatusPhase::RemoteStageWorkspace,
        None,
        "staging remote workspace",
    )?;
    let available_files = collect_workspace_files(workspace_root, &task.context)?;
    let manifest = build_current_state_manifest(available_files, &task.context);
    let staged_dir = tempfile::tempdir().context("failed to create staged remote workspace")?;
    materialize_manifest_files(workspace_root, staged_dir.path(), &manifest.entries)?;
    let archive_zip_base64 = build_zip_snapshot_base64(staged_dir.path())?;
    emit_task_status_message(
        output_observer,
        &task.label,
        1,
        TaskStatusPhase::RemoteStageWorkspace,
        None,
        format!("staged remote workspace ({} files)", manifest.entries.len()),
    )?;

    Ok(RemoteWorkspaceStage {
        temp_dir: staged_dir,
        manifest_hash: manifest.hash,
        archive_zip_base64,
    })
}

fn build_zip_snapshot_base64(staged_root: &Path) -> Result<String> {
    let mut archive_bytes = Vec::<u8>::new();
    {
        let cursor = std::io::Cursor::new(&mut archive_bytes);
        let mut zip = zip::ZipWriter::new(cursor);
        write_zip_entries_recursive(staged_root, staged_root, &mut zip)?;
        zip.finish()
            .context("failed finishing staged workspace zip snapshot")?;
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(archive_bytes))
}

fn write_zip_entries_recursive<W: Write + std::io::Seek>(
    staged_root: &Path,
    current_dir: &Path,
    zip: &mut zip::ZipWriter<W>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)
        .with_context(|| format!("failed to read staged directory {}", current_dir.display()))?
    {
        let entry = entry.with_context(|| {
            format!(
                "failed to read staged entry under {}",
                current_dir.display()
            )
        })?;
        let path = entry.path();
        let file_type = entry.file_type().with_context(|| {
            format!(
                "failed to read staged file type for {}",
                path.to_string_lossy()
            )
        })?;
        if file_type.is_dir() {
            write_zip_entries_recursive(staged_root, &path, zip)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let relative = path.strip_prefix(staged_root).with_context(|| {
            format!(
                "failed to compute staged relative path {} under {}",
                path.display(),
                staged_root.display()
            )
        })?;
        let name = normalize_filesystem_relative_path(relative);
        let options = SimpleFileOptions::default();
        zip.start_file(&name, options)
            .with_context(|| format!("failed to start zip entry {name}"))?;
        let mut file = fs::File::open(&path)
            .with_context(|| format!("failed to open staged file {}", path.display()))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .with_context(|| format!("failed to read staged file {}", path.display()))?;
        zip.write_all(&buffer)
            .with_context(|| format!("failed to write zip entry {name}"))?;
    }

    Ok(())
}
