//! The `[auto_update]` section of `agent.toml`.
//!
//! Every field is defaulted so an existing `agent.toml` (written before this
//! feature) keeps parsing and inherits the documented defaults.

use serde::{Deserialize, Serialize};

/// Where the updater is allowed to fetch release artifacts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateNetwork {
    /// No automatic fetching (the default for `tor`-only nodes).
    Disabled,
    /// Fetch from the clearnet release host over HTTPS.
    Clearnet,
}

/// Auto-update settings for a `takd` node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoUpdateConfig {
    /// Master switch / kill switch. When false the update loop never runs.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Apply updates automatically (vs check-and-log only).
    #[serde(default = "default_true")]
    pub auto_apply: bool,
    /// Where fetches may go; unset resolves from transport (see [`Self::effective_network`]).
    #[serde(default)]
    pub network: Option<UpdateNetwork>,
    /// Require a verified minisign signature before applying (authenticity gate).
    #[serde(default = "default_true")]
    pub require_signature: bool,
    /// Base hours between update checks.
    #[serde(default = "default_check_interval_hours")]
    pub check_interval_hours: u64,
    /// Max random hours added to each check (anti fleet-storm jitter).
    #[serde(default = "default_jitter_hours")]
    pub jitter_hours: u64,
    /// Release repository override (`owner/name`); unset uses the built-in default.
    #[serde(default)]
    pub repo: Option<String>,
    /// Pin a specific tag instead of tracking the latest.
    #[serde(default)]
    pub pinned_version: Option<String>,
    /// Permit installing an older version than the running one.
    #[serde(default)]
    pub allow_downgrade: bool,
    /// Also replace a co-located `tak` client binary.
    #[serde(default = "default_true")]
    pub include_sibling_tak: bool,
    /// Max seconds to wait for in-flight tasks to drain before applying.
    #[serde(default = "default_drain_timeout_secs")]
    pub drain_timeout_secs: u64,
}

impl Default for AutoUpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_apply: true,
            network: None,
            require_signature: true,
            check_interval_hours: default_check_interval_hours(),
            jitter_hours: default_jitter_hours(),
            repo: None,
            pinned_version: None,
            allow_downgrade: false,
            include_sibling_tak: true,
            drain_timeout_secs: default_drain_timeout_secs(),
        }
    }
}

impl AutoUpdateConfig {
    /// Resolve the effective network policy given the node's transport.
    ///
    /// An explicit `network` wins; otherwise `direct` nodes default to clearnet and
    /// every other transport (e.g. `tor`) defaults to disabled, so a tor-only node
    /// never silently reaches the clearnet release host.
    ///
    /// ```rust
    /// use takd::agent::{AutoUpdateConfig, UpdateNetwork};
    ///
    /// let cfg = AutoUpdateConfig::default();
    /// assert_eq!(cfg.effective_network("direct"), UpdateNetwork::Clearnet);
    /// assert_eq!(cfg.effective_network("tor"), UpdateNetwork::Disabled);
    /// ```
    pub fn effective_network(&self, transport: &str) -> UpdateNetwork {
        self.network.unwrap_or(if transport == "direct" {
            UpdateNetwork::Clearnet
        } else {
            UpdateNetwork::Disabled
        })
    }

    /// Whether the background update loop should run for a node on `transport`.
    ///
    /// ```rust
    /// use takd::agent::AutoUpdateConfig;
    ///
    /// let cfg = AutoUpdateConfig::default();
    /// assert!(cfg.loop_enabled("direct"));
    /// assert!(!cfg.loop_enabled("tor"));
    /// ```
    pub fn loop_enabled(&self, transport: &str) -> bool {
        self.enabled && self.effective_network(transport) != UpdateNetwork::Disabled
    }
}

fn default_true() -> bool {
    true
}

fn default_check_interval_hours() -> u64 {
    24
}

fn default_jitter_hours() -> u64 {
    6
}

fn default_drain_timeout_secs() -> u64 {
    1800
}
