use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use super::{durable_state, workspace};
use crate::daemon::attempt_coordinator::AttemptObservation;
use crate::daemon::run_store::RunStore;
use crate::daemon::scheduler::DispatchCommand;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrapperRequest {
    pub(super) db_path: PathBuf,
    pub(super) command: DispatchCommand,
}

pub(super) fn persist_request(
    root: &Path,
    db_path: PathBuf,
    command: &DispatchCommand,
) -> Result<PathBuf> {
    private_dir(root)?;
    let path = root.join("request.json");
    let request = WrapperRequest {
        db_path,
        command: command.clone(),
    };
    if path.try_exists()? {
        ensure!(
            read_request(&path)? == request,
            "local attempt request changed"
        );
        return Ok(path);
    }
    let temporary = root.join(format!("request-{}.tmp", uuid::Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(&serde_json::to_vec(&request)?)?;
    file.sync_all()?;
    fs::rename(&temporary, &path)?;
    fs::File::open(root)?.sync_all()?;
    ensure!(
        read_request(&path)? == request,
        "local attempt request changed"
    );
    Ok(path)
}

pub(super) fn read_request(path: &Path) -> Result<WrapperRequest> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).context("decode local attempt request")
}

pub(super) async fn dispatch(
    store: &RunStore,
    executable: &Path,
    command: &DispatchCommand,
) -> Result<()> {
    let root = store.attempt_root(command);
    let request = persist_request(&root, store.db_path().to_path_buf(), command)?;
    let mut process = wrapper_command(executable, &request)
        .spawn()
        .with_context(|| format!("launch local attempt wrapper {}", executable.display()))?;
    loop {
        if workspace::read_completion(&root)?.is_some() || root.join("started").try_exists()? {
            let _reaper = spawn_reaper(process)?;
            return Ok(());
        }
        if !store.local_attempt_is_current(command)? {
            let _reaper = spawn_reaper(process)?;
            return Ok(());
        }
        if let Some(status) = process.try_wait()? {
            if durable_state::observe(&root)? == AttemptObservation::Running {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                continue;
            }
            bail!("local attempt wrapper exited before starting: {status}");
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
}

pub(super) fn spawn_reaper(mut process: Child) -> Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("tak-local-attempt-reaper".into())
        .spawn(move || {
            if let Err(error) = process.wait() {
                tracing::debug!("reap local attempt wrapper: {error}");
            }
        })
        .context("start local attempt wrapper reaper")
}

fn wrapper_command(executable: &Path, request: &Path) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["__local-attempt", "--request"])
        .arg(request)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}

fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
