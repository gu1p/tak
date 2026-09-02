use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use tokio::time::timeout;

use super::http_client::{RemoteStream, send_worker_identity_request};
use super::startup_failure::{
    StartupTorClientRestart, StartupTorFailureDecision, StartupTorFailureTracker,
};

pub(super) fn record_startup_failure(
    tracker: &mut StartupTorFailureTracker,
    detail: &str,
) -> Result<()> {
    match tracker.record_failure(detail) {
        StartupTorFailureDecision::KeepWaiting => Ok(()),
        StartupTorFailureDecision::RestartTorClient { reason } => {
            Err(StartupTorClientRestart::new(reason).into())
        }
    }
}

pub(super) async fn probe_worker_identity(
    stream: RemoteStream,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<()> {
    let (status, body) =
        send_worker_identity_request(stream, authority, bearer_token, base_url).await?;
    if status != 200 {
        if status == 426 {
            bail!("upgrade tak, takd, and workers together");
        }
        bail!("worker identity probe failed with HTTP {status}");
    }
    tak_proto::worker_v2::decode_identity(&body).context("decode worker protocol-v2 identity")?;
    Ok(())
}

pub(super) async fn run_with_attempt_timeout<T, F, E>(
    deadline: Instant,
    max_timeout: Duration,
    stage: &str,
    future: F,
) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let attempt_timeout = deadline
        .saturating_duration_since(Instant::now())
        .min(max_timeout);
    if attempt_timeout.is_zero() {
        bail!("{stage} timed out before the attempt started");
    }
    timeout(attempt_timeout, future)
        .await
        .map_err(|_| anyhow!("{stage} timed out after {}ms", attempt_timeout.as_millis()))?
        .map_err(Into::into)
}
