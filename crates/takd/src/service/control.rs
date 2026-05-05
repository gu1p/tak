use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use prost::Message;
use tak_proto::ErrorResponse;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};

use crate::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, handle_remote_v1_http_stream};

const CONTROL_SOCKET_FILE: &str = "agent-control.sock";

#[derive(Clone, Default)]
pub(crate) struct AgentControlState {
    context: Arc<Mutex<Option<RemoteNodeContext>>>,
}

impl AgentControlState {
    pub(crate) fn set_context(&self, context: RemoteNodeContext) -> Result<()> {
        let mut guard = self
            .context
            .lock()
            .map_err(|_| anyhow!("agent control state lock poisoned"))?;
        *guard = Some(context);
        Ok(())
    }

    fn context(&self) -> Result<Option<RemoteNodeContext>> {
        self.context
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| anyhow!("agent control state lock poisoned"))
    }
}

pub(crate) fn spawn_agent_control_socket(
    state_root: &Path,
    store: SubmitAttemptStore,
    control_state: AgentControlState,
) -> Result<()> {
    let socket_path = agent_control_socket_path(state_root);
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create control socket dir {}", parent.display()))?;
    }
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)
            .with_context(|| format!("remove stale control socket {}", socket_path.display()))?;
    }
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("bind takd control socket {}", socket_path.display()))?;
    tokio::spawn(run_agent_control_socket(listener, store, control_state));
    Ok(())
}

pub fn agent_control_socket_path(state_root: &Path) -> PathBuf {
    state_root.join(CONTROL_SOCKET_FILE)
}

async fn run_agent_control_socket(
    listener: UnixListener,
    store: SubmitAttemptStore,
    control_state: AgentControlState,
) {
    loop {
        let accepted = listener.accept().await;
        let Ok((stream, _)) = accepted else {
            if let Err(err) = accepted {
                tracing::error!("takd control socket accept failed: {err}");
            }
            continue;
        };
        let store = store.clone();
        let control_state = control_state.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_agent_control_stream(stream, store, control_state).await {
                tracing::error!("takd control socket client failed: {err}");
            }
        });
    }
}

async fn handle_agent_control_stream(
    mut stream: UnixStream,
    store: SubmitAttemptStore,
    control_state: AgentControlState,
) -> Result<()> {
    let Some(context) = control_state.context()? else {
        write_status_unavailable(&mut stream).await?;
        return Ok(());
    };
    handle_remote_v1_http_stream(&mut stream, &store, &context).await
}

async fn write_status_unavailable(stream: &mut UnixStream) -> Result<()> {
    let body = ErrorResponse {
        message: "status_unavailable".to_string(),
    }
    .encode_to_vec();
    let head = format!(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&body).await?;
    Ok(())
}
