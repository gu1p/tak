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
use sha2::{Digest, Sha256};
use tak_core::model::{ResolvedTask, build_current_state_manifest};
use zip::write::SimpleFileOptions;

use super::{RemoteWorkspaceStage, TaskOutputObserver, TaskStatusPhase};

use super::TaskStatusEventKind;
use super::output_observer::{TaskStatusDetails, emit_task_status_message_with_details};
use super::remote_models::format_upload_size_mb;
use super::workspace_collect::{collect_workspace_files, materialize_manifest_files};
use super::workspace_sync::normalize_filesystem_relative_path;

pub(crate) fn stage_remote_workspace(
    task: &ResolvedTask,
    workspace_root: &Path,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<RemoteWorkspaceStage> {
    emit_task_status_message_with_details(
        output_observer,
        &task.label,
        1,
        TaskStatusPhase::RemoteStageWorkspace,
        None,
        "staging remote workspace",
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::WorkspaceStage),
            ..TaskStatusDetails::default()
        },
    )?;
    let available_files = collect_workspace_files(workspace_root, &task.context)?;
    let manifest = build_current_state_manifest(available_files, &task.context);
    let staged_dir = tempfile::tempdir().context("failed to create staged remote workspace")?;
    let staged_files_dir = staged_dir.path().join("files");
    fs::create_dir_all(&staged_files_dir).context("failed to create staged files directory")?;
    materialize_manifest_files(workspace_root, &staged_files_dir, &manifest.entries)?;
    let archive_path = staged_dir.path().join("workspace.zip");
    let (archive_byte_len, sha256) = write_zip_snapshot_hashed(&staged_files_dir, &archive_path)?;
    emit_task_status_message_with_details(
        output_observer,
        &task.label,
        1,
        TaskStatusPhase::RemoteStageWorkspace,
        None,
        format!(
            "staged remote workspace ({} files, {} upload)",
            manifest.entries.len(),
            format_upload_size_mb(archive_byte_len)
        ),
        TaskStatusDetails {
            kind: Some(TaskStatusEventKind::WorkspaceStage),
            bytes_total: Some(archive_byte_len),
            ..TaskStatusDetails::default()
        },
    )?;

    Ok(RemoteWorkspaceStage {
        temp_dir: staged_dir,
        archive_byte_len,
        archive_path,
        sha256,
    })
}

pub(super) fn write_zip_snapshot_hashed(
    staged_root: &Path,
    archive_path: &Path,
) -> Result<(u64, String)> {
    let archive = fs::File::create(archive_path)
        .with_context(|| format!("failed to create {}", archive_path.display()))?;
    let writer = std::io::BufWriter::new(archive);
    let mut zip = zip::ZipWriter::new(writer);
    write_zip_entries_recursive(staged_root, staged_root, &mut zip)?;
    let mut writer = zip
        .finish()
        .context("failed finishing staged workspace zip snapshot")?;
    writer
        .flush()
        .context("failed flushing staged workspace zip snapshot")?;
    let byte_len = fs::metadata(archive_path)
        .with_context(|| format!("failed to stat {}", archive_path.display()))?
        .len();
    let sha256 = hash_file_hex(archive_path)?;
    Ok((byte_len, sha256))
}

fn hash_file_hex(path: &Path) -> Result<String> {
    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
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
