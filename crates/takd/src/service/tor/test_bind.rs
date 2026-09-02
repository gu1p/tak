use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tokio::time::sleep;

use crate::agent::{
    TransportHealth, persist_ready_base_url, ready_context_with_state_root, write_transport_health,
};
use crate::daemon::remote::{SubmitAttemptStore, run_worker_http_server};
use crate::service::control::AgentControlState;

use super::TorSessionExit;
use super::health::{take_test_force_recovery_after, take_test_startup_failure};
use super::live_state::recovering_exit;
use super::ready_config;

#[derive(Debug)]
pub(super) struct RetryableTestBindStartupFailure;

impl std::fmt::Display for RetryableTestBindStartupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("test startup failure hook triggered")
    }
}

impl std::error::Error for RetryableTestBindStartupFailure {}

pub(super) async fn serve_test_bind_session(
    config_root: &std::path::Path,
    state_root: &std::path::Path,
    config: &crate::agent::AgentConfig,
    nickname: &str,
    store: SubmitAttemptStore,
    bind_addr: &str,
    control_state: AgentControlState,
) -> Result<TorSessionExit> {
    if take_test_startup_failure(state_root) {
        return Err(RetryableTestBindStartupFailure.into());
    }
    let base_url = format!("http://{nickname}.onion");
    tracing::info!("using takd tor hidden-service test bind override at {bind_addr}");
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind takd tor test listener at {bind_addr}"))?;
    persist_ready_base_url(config_root, state_root, &base_url)?;
    write_transport_health(state_root, &TransportHealth::ready(Some(base_url.clone())))?;
    tracing::info!("takd worker v2 onion service ready at {}", base_url);
    let context = ready_context_with_state_root(&ready_config(config, &base_url), state_root)?;
    let context = control_state.set_context(context)?;

    if let Some(delay) = take_test_force_recovery_after(state_root) {
        let mut server = tokio::spawn(run_worker_http_server(listener, store, context.clone()));
        tokio::select! {
            result = &mut server => match result {
                Ok(Ok(())) => recovering_exit(
                    &context,
                    state_root,
                    &base_url,
                    "takd tor test-bind server stopped",
                ),
                Ok(Err(err)) => Err(err),
                Err(join_err) => Err(join_err).context("takd tor test-bind task failed"),
            },
            _ = sleep(delay) => {
                tracing::warn!(
                    "forcing takd tor recovery in test-bind mode after {}ms",
                    delay.as_millis()
                );
                let exit = recovering_exit(
                    &context,
                    state_root,
                    &base_url,
                    "test recovery hook triggered",
                )?;
                server.abort();
                let _ = server.await;
                Ok(exit)
            }
        }
    } else {
        run_worker_http_server(listener, store, context.clone()).await?;
        recovering_exit(
            &context,
            state_root,
            &base_url,
            "takd tor test-bind server stopped",
        )
    }
}
