use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::paths::transport_health_path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportState {
    Pending,
    Ready,
    Recovering,
}

impl TransportState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Ready => "ready",
            Self::Recovering => "recovering",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransportHealth {
    pub transport_state: TransportState,
    pub base_url: Option<String>,
    pub detail: Option<String>,
}

impl TransportHealth {
    pub fn new(
        transport_state: TransportState,
        base_url: Option<String>,
        detail: Option<String>,
    ) -> Self {
        Self {
            transport_state,
            base_url,
            detail,
        }
    }

    pub fn pending(base_url: Option<String>) -> Self {
        Self::new(TransportState::Pending, base_url, None)
    }

    pub fn ready(base_url: Option<String>) -> Self {
        Self::new(TransportState::Ready, base_url, None)
    }

    pub fn recovering(base_url: Option<String>, detail: Option<String>) -> Self {
        Self::new(TransportState::Recovering, base_url, detail)
    }
}

pub fn write_transport_health(state_root: &Path, health: &TransportHealth) -> Result<()> {
    fs::create_dir_all(state_root)
        .with_context(|| format!("create transport health dir {}", state_root.display()))?;
    let path = transport_health_path(state_root);
    fs::write(
        &path,
        toml::to_string(health).context("encode transport health")?,
    )
    .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn read_transport_health(state_root: &Path) -> Result<Option<TransportHealth>> {
    let path = transport_health_path(state_root);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
    };
    Ok(Some(
        toml::from_str(&raw).with_context(|| format!("decode {}", path.display()))?,
    ))
}

#[derive(Debug, Clone)]
pub struct TorRecoveryTracker {
    failure_threshold: u32,
    consecutive_failures: u32,
}

impl TorRecoveryTracker {
    pub fn new(failure_threshold: u32) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            consecutive_failures: 0,
        }
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub fn record_failure(&mut self) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.consecutive_failures >= self.failure_threshold
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }
}

#[derive(Debug, Clone)]
pub struct TorRecoveryBackoff {
    initial: Duration,
    current: Duration,
    max: Duration,
}

impl TorRecoveryBackoff {
    pub fn new(initial: Duration, max: Duration) -> Self {
        let initial = if initial.is_zero() {
            Duration::from_millis(1)
        } else {
            initial
        };
        let max = max.max(initial);
        Self {
            initial,
            current: initial,
            max,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let delay = self.current;
        self.current = self.current.saturating_mul(2).min(self.max);
        delay
    }

    pub fn reset(&mut self) {
        self.current = self.initial;
    }
}
