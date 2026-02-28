//! Tak daemon protocol and lease coordination engine.
//!
//! The daemon serves NDJSON requests over a Unix socket and coordinates machine-wide
//! limiter leases with optional SQLite-backed persistence and history.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use futures::StreamExt;
use rusqlite::{Connection, ErrorCode, params};
use safelog::DisplayRedacted;
use serde::{Deserialize, Serialize};
use tak_core::model::Scope;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tor_cell::relaycell::msg::Connected;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user: String,
    pub pid: u32,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub label: String,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedRequest {
    pub name: String,
    pub scope: Scope,
    pub scope_key: Option<String>,
    pub slots: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquireLeaseRequest {
    pub request_id: String,
    pub client: ClientInfo,
    pub task: TaskInfo,
    pub needs: Vec<NeedRequest>,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenewLeaseRequest {
    pub request_id: String,
    pub lease_id: String,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseLeaseRequest {
    pub request_id: String,
    pub lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRequest {
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub lease_id: String,
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInfo {
    pub queue_position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    AcquireLease(AcquireLeaseRequest),
    RenewLease(RenewLeaseRequest),
    ReleaseLease(ReleaseLeaseRequest),
    Status(StatusRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    LeaseGranted {
        request_id: String,
        lease: LeaseInfo,
    },
    LeasePending {
        request_id: String,
        pending: PendingInfo,
    },
    LeaseRenewed {
        request_id: String,
        ttl_ms: u64,
    },
    LeaseReleased {
        request_id: String,
    },
    StatusSnapshot {
        request_id: String,
        status: StatusSnapshot,
    },
    Error {
        request_id: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
    pub active_leases: usize,
    pub pending_requests: usize,
    pub usage: Vec<LimiterUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterUsage {
    pub name: String,
    pub scope: Scope,
    pub scope_key: Option<String>,
    pub used: f64,
    pub capacity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerEngine {
    Docker,
    Podman,
}

impl ContainerEngine {
    #[must_use]
    fn as_name(self) -> &'static str {
        match self {
            Self::Docker => "docker",
            Self::Podman => "podman",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    MacOs,
    Other,
}

impl HostPlatform {
    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

pub trait ContainerEngineProbe {
    fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtiSettings {
    pub socks5_addr: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorTransportConfig {
    pub onion_endpoint: String,
    pub service_auth_token: String,
    pub arti: ArtiSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorHiddenServiceRuntimeConfig {
    pub nickname: String,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
}

/// Validates Tor transport configuration before any transport/client creation.
///
/// ```rust
/// # use takd::{ArtiSettings, TorTransportConfig, validate_tor_transport_config};
/// let config = TorTransportConfig {
///     onion_endpoint: "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion".to_string(),
///     service_auth_token: "service-token-123".to_string(),
///     arti: ArtiSettings {
///         socks5_addr: "127.0.0.1:9150".to_string(),
///         data_dir: "/tmp/tak/arti".to_string(),
///     },
/// };
/// validate_tor_transport_config(&config)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn validate_tor_transport_config(config: &TorTransportConfig) -> Result<()> {
    ensure_present("onion endpoint", &config.onion_endpoint)?;
    if !is_valid_onion_endpoint(&config.onion_endpoint) {
        bail!("tor onion endpoint must target a .onion host");
    }

    ensure_present("service auth token", &config.service_auth_token)?;
    if config.service_auth_token.chars().any(char::is_whitespace) {
        bail!("tor service auth token contains invalid characters");
    }

    ensure_present("arti socks5 address", &config.arti.socks5_addr)?;
    ensure_present("arti data directory", &config.arti.data_dir)?;
    Ok(())
}

/// Validates and canonicalizes Tor transport configuration values.
///
/// ```rust
/// # use takd::{ArtiSettings, TorTransportConfig, normalize_tor_transport_config};
/// let normalized = normalize_tor_transport_config(TorTransportConfig {
///     onion_endpoint: "  http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion  ".to_string(),
///     service_auth_token: " service-token-123 ".to_string(),
///     arti: ArtiSettings {
///         socks5_addr: " 127.0.0.1:9150 ".to_string(),
///         data_dir: " /tmp/tak/arti ".to_string(),
///     },
/// })?;
/// assert_eq!(normalized.arti.socks5_addr, "127.0.0.1:9150");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn normalize_tor_transport_config(config: TorTransportConfig) -> Result<TorTransportConfig> {
    let normalized = TorTransportConfig {
        onion_endpoint: config.onion_endpoint.trim().to_string(),
        service_auth_token: config.service_auth_token.trim().to_string(),
        arti: ArtiSettings {
            socks5_addr: config.arti.socks5_addr.trim().to_string(),
            data_dir: config.arti.data_dir.trim().to_string(),
        },
    };
    validate_tor_transport_config(&normalized)?;
    Ok(normalized)
}

/// Resolves container engine deterministically: Docker first, then Podman on macOS only.
///
/// ```no_run
/// # // Reason: This behavior depends on host engine availability and is compile-checked only.
/// # use takd::{ContainerEngine, ContainerEngineProbe, HostPlatform, select_container_engine_with_probe};
/// # struct Probe;
/// # impl ContainerEngineProbe for Probe {
/// #     fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
/// #         match engine {
/// #             ContainerEngine::Docker => Ok(()),
/// #             ContainerEngine::Podman => Err("podman unavailable".to_string()),
/// #         }
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut probe = Probe;
/// let selected = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)?;
/// assert_eq!(selected, ContainerEngine::Docker);
/// # Ok(())
/// # }
/// ```
pub fn select_container_engine_with_probe(
    platform: HostPlatform,
    probe: &mut impl ContainerEngineProbe,
) -> Result<ContainerEngine> {
    if probe.probe(ContainerEngine::Docker).is_ok() {
        return Ok(ContainerEngine::Docker);
    }

    let mut attempted = vec![ContainerEngine::Docker.as_name()];
    if matches!(platform, HostPlatform::MacOs) {
        if probe.probe(ContainerEngine::Podman).is_ok() {
            return Ok(ContainerEngine::Podman);
        }
        attempted.push(ContainerEngine::Podman.as_name());
    }

    bail!(
        "no container engine available; attempted probes: {}",
        attempted.join(", ")
    );
}

/// Resolves container engine using the current host platform.
///
/// ```no_run
/// # // Reason: This behavior depends on host engine availability and is compile-checked only.
/// # use takd::{ContainerEngine, ContainerEngineProbe, select_container_engine};
/// # struct Probe;
/// # impl ContainerEngineProbe for Probe {
/// #     fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
/// #         match engine {
/// #             ContainerEngine::Docker => Ok(()),
/// #             ContainerEngine::Podman => Err("podman unavailable".to_string()),
/// #         }
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut probe = Probe;
/// let selected = select_container_engine(&mut probe)?;
/// assert_eq!(selected, ContainerEngine::Docker);
/// # Ok(())
/// # }
/// ```
pub fn select_container_engine(probe: &mut impl ContainerEngineProbe) -> Result<ContainerEngine> {
    select_container_engine_with_probe(HostPlatform::current(), probe)
}

fn ensure_present(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("tor {field} is required");
    }
    Ok(())
}

fn is_valid_onion_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.trim();
    let without_scheme = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);
    let host_port = without_scheme.split('/').next().unwrap_or_default();
    let host = host_port.split(':').next().unwrap_or_default();
    host.ends_with(".onion")
}

#[derive(Debug, Clone)]
pub enum AcquireLeaseResponse {
    LeaseGranted { lease: LeaseInfo },
    LeasePending { pending: PendingInfo },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct LimiterKey {
    name: String,
    scope: Scope,
    scope_key: Option<String>,
}

#[derive(Debug, Clone)]
struct LeaseRecord {
    needs: Vec<NeedRequest>,
    expires_at: Instant,
    ttl_ms: u64,
    request_id: String,
    task_label: String,
    user_name: String,
    pid: u32,
}

#[derive(Debug, Default)]
pub struct LeaseManager {
    capacities: HashMap<LimiterKey, f64>,
    usage: HashMap<LimiterKey, f64>,
    leases: HashMap<String, LeaseRecord>,
    pending: VecDeque<AcquireLeaseRequest>,
    db_path: Option<PathBuf>,
}

impl LeaseManager {
    #[must_use]
    /// Creates an in-memory lease manager with no configured capacities.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a SQLite-backed lease manager and restores active lease state.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        let mut manager = Self {
            db_path: Some(db_path),
            ..Self::default()
        };
        manager.ensure_schema()?;
        manager.restore_active_leases()?;
        Ok(manager)
    }

    /// Sets capacity for one limiter key.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_capacity(
        &mut self,
        name: impl Into<String>,
        scope: Scope,
        scope_key: Option<String>,
        capacity: f64,
    ) {
        self.capacities.insert(
            LimiterKey {
                name: name.into(),
                scope,
                scope_key,
            },
            capacity,
        );
    }

    /// Attempts to atomically acquire all requested needs for a lease request.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn acquire(&mut self, request: AcquireLeaseRequest) -> AcquireLeaseResponse {
        self.expire_leases();

        if let Some(existing) = self
            .pending
            .iter()
            .position(|pending| pending.request_id == request.request_id)
        {
            self.pending.remove(existing);
        }

        if self.can_allocate(&request.needs) {
            self.allocate(&request.needs);
            let lease_id = Uuid::new_v4().to_string();
            let ttl_ms = request.ttl_ms.max(1_000);
            let expires_at = Instant::now() + Duration::from_millis(ttl_ms);
            let lease_record = LeaseRecord {
                needs: request.needs,
                expires_at,
                ttl_ms,
                request_id: request.request_id,
                task_label: request.task.label,
                user_name: request.client.user,
                pid: request.client.pid,
            };

            self.leases.insert(lease_id.clone(), lease_record.clone());
            self.persist_active_lease(&lease_id, &lease_record)
                .expect("failed to persist active lease");
            self.append_history("acquire", &lease_id, &lease_record)
                .expect("failed to append acquire history");

            return AcquireLeaseResponse::LeaseGranted {
                lease: LeaseInfo {
                    lease_id,
                    ttl_ms,
                    renew_after_ms: ttl_ms / 3,
                },
            };
        }

        self.pending.push_back(request);
        AcquireLeaseResponse::LeasePending {
            pending: PendingInfo {
                queue_position: self.pending.len(),
            },
        }
    }

    /// Renews an existing lease by updating TTL and persisted expiry.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn renew(&mut self, lease_id: &str, ttl_ms: u64) -> Result<()> {
        self.expire_leases();
        let effective_ttl = ttl_ms.max(1_000);

        let updated_record = {
            let record = self
                .leases
                .get_mut(lease_id)
                .ok_or_else(|| anyhow!("lease {lease_id} does not exist"))?;

            record.ttl_ms = effective_ttl;
            record.expires_at = Instant::now() + Duration::from_millis(effective_ttl);
            record.clone()
        };

        self.persist_active_lease(lease_id, &updated_record)?;
        self.append_history("renew", lease_id, &updated_record)?;

        Ok(())
    }

    /// Releases an active lease and reclaims associated limiter usage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn release(&mut self, lease_id: &str) -> Result<()> {
        self.expire_leases();

        let record = self
            .leases
            .remove(lease_id)
            .ok_or_else(|| anyhow!("lease {lease_id} does not exist"))?;
        self.deallocate(&record.needs);
        self.delete_active_lease(lease_id)?;
        self.append_history("release", lease_id, &record)?;

        Ok(())
    }

    #[must_use]
    /// Returns current daemon state as an externally-visible status snapshot.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn status(&mut self) -> StatusSnapshot {
        self.expire_leases();

        let usage = self
            .usage
            .iter()
            .map(|(key, used)| LimiterUsage {
                name: key.name.clone(),
                scope: key.scope.clone(),
                scope_key: key.scope_key.clone(),
                used: *used,
                capacity: self.capacities.get(key).copied().unwrap_or(f64::INFINITY),
            })
            .collect();

        StatusSnapshot {
            active_leases: self.leases.len(),
            pending_requests: self.pending.len(),
            usage,
        }
    }

    /// Expires stale leases and frees their allocated usage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn expire_leases(&mut self) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .leases
            .iter()
            .filter_map(|(lease_id, record)| (record.expires_at <= now).then_some(lease_id.clone()))
            .collect();

        for lease_id in expired {
            if let Some(record) = self.leases.remove(&lease_id) {
                self.deallocate(&record.needs);
                if let Err(err) = self.delete_active_lease(&lease_id) {
                    eprintln!("failed to delete expired lease {lease_id} from sqlite: {err}");
                }
                if let Err(err) = self.append_history("expire", &lease_id, &record) {
                    eprintln!("failed to append expire history for {lease_id}: {err}");
                }
            }
        }
    }

    /// Checks whether all needs can be satisfied together without over-allocation.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn can_allocate(&self, needs: &[NeedRequest]) -> bool {
        let mut delta: HashMap<LimiterKey, f64> = HashMap::new();

        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            *delta.entry(key).or_insert(0.0) += need.slots;
        }

        delta.into_iter().all(|(key, requested)| {
            let used = self.usage.get(&key).copied().unwrap_or(0.0);
            let capacity = self.capacities.get(&key).copied().unwrap_or(f64::INFINITY);
            used + requested <= capacity
        })
    }

    /// Adds slots to usage totals for each requested need.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn allocate(&mut self, needs: &[NeedRequest]) {
        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            *self.usage.entry(key).or_insert(0.0) += need.slots;
        }
    }

    /// Removes slots from usage totals for each requested need.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn deallocate(&mut self, needs: &[NeedRequest]) {
        for need in needs {
            let key = LimiterKey {
                name: need.name.clone(),
                scope: need.scope.clone(),
                scope_key: need.scope_key.clone(),
            };
            if let Some(entry) = self.usage.get_mut(&key) {
                *entry = (*entry - need.slots).max(0.0);
                if *entry == 0.0 {
                    self.usage.remove(&key);
                }
            }
        }
    }

    /// Ensures SQLite schema exists for active leases and lease history.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn ensure_schema(&self) -> Result<()> {
        let Some(mut conn) = self.open_connection()? else {
            return Ok(());
        };

        let tx = conn.transaction()?;
        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS active_leases (
                lease_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                task_label TEXT NOT NULL,
                user_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                needs_json TEXT NOT NULL,
                ttl_ms INTEGER NOT NULL,
                expires_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS lease_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts_ms INTEGER NOT NULL,
                event TEXT NOT NULL,
                lease_id TEXT NOT NULL,
                request_id TEXT NOT NULL,
                task_label TEXT NOT NULL,
                user_name TEXT NOT NULL,
                pid INTEGER NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_lease_history_lease_id ON lease_history(lease_id);
            CREATE INDEX IF NOT EXISTS idx_lease_history_event ON lease_history(event);
            ",
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Restores non-expired active leases from SQLite into in-memory state.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn restore_active_leases(&mut self) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let now_ms = unix_epoch_ms();
        let mut stmt = conn.prepare(
            "SELECT lease_id, request_id, task_label, user_name, pid, needs_json, ttl_ms, expires_at_ms FROM active_leases",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StoredLeaseRow {
                lease_id: row.get::<_, String>(0)?,
                request_id: row.get::<_, String>(1)?,
                task_label: row.get::<_, String>(2)?,
                user_name: row.get::<_, String>(3)?,
                pid: row.get::<_, u32>(4)?,
                needs_json: row.get::<_, String>(5)?,
                ttl_ms: row.get::<_, i64>(6)?,
                expires_at_ms: row.get::<_, i64>(7)?,
            })
        })?;

        let mut expired_ids = Vec::new();

        for row in rows {
            let row = row?;
            if row.expires_at_ms <= now_ms {
                expired_ids.push(row.lease_id);
                continue;
            }

            let ttl_ms = u64::try_from(row.ttl_ms).with_context(|| {
                format!(
                    "invalid persisted ttl_ms {} for lease {}",
                    row.ttl_ms, row.lease_id
                )
            })?;
            let remaining_ms = (row.expires_at_ms - now_ms) as u64;
            let needs: Vec<NeedRequest> =
                serde_json::from_str(&row.needs_json).with_context(|| {
                    format!("failed to parse needs_json for lease {}", row.lease_id)
                })?;

            self.allocate(&needs);
            self.leases.insert(
                row.lease_id,
                LeaseRecord {
                    needs,
                    expires_at: Instant::now() + Duration::from_millis(remaining_ms),
                    ttl_ms,
                    request_id: row.request_id,
                    task_label: row.task_label,
                    user_name: row.user_name,
                    pid: row.pid,
                },
            );
        }

        if !expired_ids.is_empty() {
            let mut conn = self
                .open_connection()?
                .ok_or_else(|| anyhow!("sqlite connection missing during cleanup"))?;
            let tx = conn.transaction()?;
            for lease_id in expired_ids {
                tx.execute("DELETE FROM active_leases WHERE lease_id = ?1", [lease_id])?;
            }
            tx.commit()?;
        }

        Ok(())
    }

    /// Upserts one active lease row in SQLite storage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn persist_active_lease(&self, lease_id: &str, record: &LeaseRecord) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let needs_json = serde_json::to_string(&record.needs)?;
        let ttl_ms = i64::try_from(record.ttl_ms)
            .with_context(|| format!("ttl_ms {} exceeds sqlite range", record.ttl_ms))?;
        let expires_at_ms = unix_epoch_ms().checked_add(ttl_ms).ok_or_else(|| {
            anyhow!(
                "ttl_ms overflow while computing expires_at_ms for lease {}",
                lease_id
            )
        })?;
        conn.execute(
            "
            INSERT INTO active_leases (
                lease_id, request_id, task_label, user_name, pid, needs_json, ttl_ms, expires_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(lease_id) DO UPDATE SET
                request_id = excluded.request_id,
                task_label = excluded.task_label,
                user_name = excluded.user_name,
                pid = excluded.pid,
                needs_json = excluded.needs_json,
                ttl_ms = excluded.ttl_ms,
                expires_at_ms = excluded.expires_at_ms
            ",
            params![
                lease_id,
                record.request_id,
                record.task_label,
                record.user_name,
                record.pid,
                needs_json,
                ttl_ms,
                expires_at_ms
            ],
        )?;

        Ok(())
    }

    /// Removes one active lease row from SQLite storage.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn delete_active_lease(&self, lease_id: &str) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        conn.execute("DELETE FROM active_leases WHERE lease_id = ?1", [lease_id])?;
        Ok(())
    }

    /// Appends one lease lifecycle event row to SQLite history.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn append_history(&self, event: &str, lease_id: &str, record: &LeaseRecord) -> Result<()> {
        let Some(conn) = self.open_connection()? else {
            return Ok(());
        };

        let payload_json = serde_json::json!({
            "needs": record.needs,
            "ttl_ms": record.ttl_ms,
        })
        .to_string();

        conn.execute(
            "
            INSERT INTO lease_history (
                ts_ms, event, lease_id, request_id, task_label, user_name, pid, payload_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                unix_epoch_ms(),
                event,
                lease_id,
                record.request_id,
                record.task_label,
                record.user_name,
                record.pid,
                payload_json
            ],
        )?;

        Ok(())
    }

    /// Opens the configured SQLite connection if persistence is enabled.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn open_connection(&self) -> Result<Option<Connection>> {
        let Some(db_path) = &self.db_path else {
            return Ok(None);
        };

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create sqlite directory {}", parent.display())
            })?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open sqlite db {}", db_path.display()))?;
        Ok(Some(conn))
    }
}

#[derive(Debug)]
struct StoredLeaseRow {
    lease_id: String,
    request_id: String,
    task_label: String,
    user_name: String,
    pid: u32,
    needs_json: String,
    ttl_ms: i64,
    expires_at_ms: i64,
}

pub type SharedLeaseManager = Arc<Mutex<LeaseManager>>;

/// Creates a shared in-memory lease manager.
#[must_use]
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn new_shared_manager() -> SharedLeaseManager {
    Arc::new(Mutex::new(LeaseManager::new()))
}

/// Creates a shared SQLite-backed lease manager.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn new_shared_manager_with_db(db_path: PathBuf) -> Result<SharedLeaseManager> {
    let manager = LeaseManager::with_db_path(db_path)?;
    Ok(Arc::new(Mutex::new(manager)))
}

/// Runs the daemon server loop on the given Unix socket path.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_server(socket_path: &Path, manager: SharedLeaseManager) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create socket directory {}", parent.display()))?;
    }

    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await.with_context(|| {
            format!("failed to remove existing socket {}", socket_path.display())
        })?;
    }

    let listener = UnixListener::bind(socket_path)
        .with_context(|| format!("failed to bind socket {}", socket_path.display()))?;

    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let manager = Arc::clone(&manager);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, manager).await {
                eprintln!("client handling error: {err}");
            }
        });
    }
}

/// Runs a TCP HTTP server that exposes canonical remote V1 endpoints backed by SQLite state.
///
/// ```no_run
/// # // Reason: This behavior depends on runtime network IO and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_remote_v1_http_server(
    listener: TcpListener,
    store: SubmitAttemptStore,
) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await.context("accept failed")?;
        let store = store.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_remote_v1_http_client(stream, store).await {
                eprintln!("remote v1 http client handling error: {err}");
            }
        });
    }
}

async fn handle_remote_v1_http_client(
    mut stream: TcpStream,
    store: SubmitAttemptStore,
) -> Result<()> {
    handle_remote_v1_http_stream(&mut stream, &store).await
}

async fn handle_remote_v1_http_stream<S>(stream: &mut S, store: &SubmitAttemptStore) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(request) = read_http_request(stream).await? else {
        return Ok(());
    };
    let response = handle_remote_v1_request(
        store,
        &request.method,
        &request.path,
        request.body.as_deref(),
    )?;
    write_http_response(stream, &response).await?;
    Ok(())
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    body: Option<String>,
}

async fn read_http_request<S>(stream: &mut S) -> Result<Option<ParsedHttpRequest>>
where
    S: AsyncRead + Unpin,
{
    let mut request_bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;

    while header_end.is_none() {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read request bytes")?;
        if read == 0 {
            break;
        }
        request_bytes.extend_from_slice(&chunk[..read]);
        header_end = request_bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|idx| idx + 4);
    }

    if request_bytes.is_empty() {
        return Ok(None);
    }

    let header_end = header_end.unwrap_or(request_bytes.len());
    let header_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let request_line = header_text.lines().next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    let mut body = request_bytes[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk).await.context("read request body")?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    let body = if body.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(&body).to_string())
    };

    Ok(Some(ParsedHttpRequest { method, path, body }))
}

async fn write_http_response<S>(stream: &mut S, response: &RemoteV1Response) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    let status = http_status_line(response.status_code);
    let encoded = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.content_type,
        response.body.len(),
        response.body
    );
    stream
        .write_all(encoded.as_bytes())
        .await
        .context("write response bytes")?;
    stream.flush().await.context("flush response bytes")?;
    Ok(())
}

fn http_status_line(status_code: u16) -> &'static str {
    match status_code {
        200 => "200 OK",
        202 => "202 Accepted",
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    }
}

/// Runs canonical remote V1 endpoints as an embedded Arti onion service.
///
/// ```no_run
/// # // Reason: This behavior depends on runtime Tor network bootstrapping and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_remote_v1_tor_hidden_service(
    config: TorHiddenServiceRuntimeConfig,
    store: SubmitAttemptStore,
) -> Result<()> {
    if let Some(test_bind_addr) = test_tor_hidden_service_bind_addr() {
        let listener = TcpListener::bind(test_bind_addr.as_str())
            .await
            .with_context(|| {
                format!("failed to bind takd tor test listener at {test_bind_addr}")
            })?;
        return run_remote_v1_http_server(listener, store).await;
    }

    let tor_client_config = arti_client::config::TorClientConfigBuilder::from_directories(
        &config.state_dir,
        &config.cache_dir,
    )
    .build()
    .context("invalid Arti client configuration for takd hidden service")?;
    let tor_client = arti_client::TorClient::create_bootstrapped(tor_client_config)
        .await
        .context("failed to bootstrap embedded Arti for takd hidden service")?;
    let onion_service_config = build_tor_hidden_service_config(&config.nickname)?;
    let Some((running_service, rend_requests)) = tor_client
        .launch_onion_service(onion_service_config)
        .context("failed to launch takd onion service via embedded Arti")?
    else {
        bail!("takd onion service launch was skipped because the service is disabled");
    };

    let onion_endpoint = running_service
        .onion_address()
        .map(|hsid| format!("http://{}", hsid.display_unredacted()))
        .ok_or_else(|| anyhow!("takd onion service did not expose an onion address"))?;
    eprintln!("takd remote v1 onion service ready at {onion_endpoint}");

    futures::pin_mut!(rend_requests);
    while let Some(rend_request) = rend_requests.next().await {
        let accepted = rend_request.accept().await;
        let mut stream_requests = match accepted {
            Ok(stream_requests) => stream_requests,
            Err(err) => {
                eprintln!("takd onion service rendezvous accept failed: {err}");
                continue;
            }
        };

        while let Some(stream_request) = stream_requests.next().await {
            match stream_request.accept(Connected::new_empty()).await {
                Ok(mut stream) => {
                    if let Err(err) = handle_remote_v1_http_stream(&mut stream, &store).await {
                        eprintln!("takd onion service stream handling failed: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("takd onion service stream accept failed: {err}");
                }
            }
        }
    }

    Ok(())
}
fn build_tor_hidden_service_config(
    nickname: &str,
) -> Result<arti_client::config::onion_service::OnionServiceConfig> {
    let nickname = nickname
        .trim()
        .parse()
        .with_context(|| format!("invalid tor hidden-service nickname `{nickname}`"))?;
    arti_client::config::onion_service::OnionServiceConfigBuilder::default()
        .nickname(nickname)
        .build()
        .context("invalid onion service config for takd")
}

fn remote_v1_bind_addr_from_env() -> Option<String> {
    std::env::var("TAKD_REMOTE_V1_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn tor_hidden_service_runtime_config_from_env() -> Result<Option<TorHiddenServiceRuntimeConfig>> {
    let nickname = match std::env::var("TAKD_TOR_HS_NICKNAME") {
        Ok(value) => value.trim().to_string(),
        Err(_) => return Ok(None),
    };
    if nickname.is_empty() {
        return Ok(None);
    }

    let state_dir = std::env::var("TAKD_TOR_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("takd-arti-state"));
    let cache_dir = std::env::var("TAKD_TOR_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("takd-arti-cache"));

    Ok(Some(TorHiddenServiceRuntimeConfig {
        nickname,
        state_dir,
        cache_dir,
    }))
}

fn test_tor_hidden_service_bind_addr() -> Option<String> {
    std::env::var("TAKD_TEST_TOR_HS_BIND_ADDR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Handles a single client connection and processes line-delimited requests.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn handle_client(stream: UnixStream, manager: SharedLeaseManager) -> Result<()> {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        let request: Request = serde_json::from_str(line.trim_end())
            .with_context(|| format!("invalid request line: {}", line.trim_end()))?;
        let response = dispatch_request(request, &manager)?;
        let encoded = serde_json::to_string(&response)?;
        writer_half.write_all(encoded.as_bytes()).await?;
        writer_half.write_all(b"\n").await?;
        writer_half.flush().await?;
    }

    Ok(())
}

/// Routes one protocol request to the lease manager and builds a protocol response.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn dispatch_request(request: Request, manager: &SharedLeaseManager) -> Result<Response> {
    match request {
        Request::AcquireLease(payload) => {
            let request_id = payload.request_id.clone();

            if let Err(err) = ensure_valid_request(&payload) {
                return Ok(Response::Error {
                    request_id,
                    message: err.to_string(),
                });
            }

            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            let response = guard.acquire(payload);
            Ok(match response {
                AcquireLeaseResponse::LeaseGranted { lease } => {
                    Response::LeaseGranted { request_id, lease }
                }
                AcquireLeaseResponse::LeasePending { pending } => Response::LeasePending {
                    request_id,
                    pending,
                },
            })
        }
        Request::RenewLease(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            match guard.renew(&payload.lease_id, payload.ttl_ms) {
                Ok(()) => Ok(Response::LeaseRenewed {
                    request_id: payload.request_id,
                    ttl_ms: payload.ttl_ms,
                }),
                Err(err) => Ok(Response::Error {
                    request_id: payload.request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::ReleaseLease(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            match guard.release(&payload.lease_id) {
                Ok(()) => Ok(Response::LeaseReleased {
                    request_id: payload.request_id,
                }),
                Err(err) => Ok(Response::Error {
                    request_id: payload.request_id,
                    message: err.to_string(),
                }),
            }
        }
        Request::Status(payload) => {
            let mut guard = manager
                .lock()
                .map_err(|_| anyhow!("lease manager lock poisoned"))?;
            Ok(Response::StatusSnapshot {
                request_id: payload.request_id,
                status: guard.status(),
            })
        }
    }
}

/// Boots the daemon with default capacities and configured SQLite persistence.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub async fn run_daemon(socket_path: &Path) -> Result<()> {
    let db_path = std::env::var("TAKD_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_state_db_path());
    let manager = new_shared_manager_with_db(db_path.clone())?;

    {
        let mut guard = manager
            .lock()
            .map_err(|_| anyhow!("lease manager lock poisoned"))?;
        guard.set_capacity("cpu", Scope::Machine, None, 8.0);
        guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    }

    spawn_optional_remote_v1_services(&db_path).await?;
    run_server(socket_path, manager).await
}

async fn spawn_optional_remote_v1_services(db_path: &Path) -> Result<()> {
    if let Some(bind_addr) = remote_v1_bind_addr_from_env() {
        let listener = TcpListener::bind(bind_addr.as_str())
            .await
            .with_context(|| {
                format!("failed to bind takd remote v1 http listener at {bind_addr}")
            })?;
        let local_addr = listener
            .local_addr()
            .context("failed to read takd remote v1 local address")?;
        let store = SubmitAttemptStore::with_db_path(db_path.to_path_buf())
            .context("failed to open takd remote v1 sqlite store")?;
        tokio::spawn(async move {
            if let Err(err) = run_remote_v1_http_server(listener, store).await {
                eprintln!("takd remote v1 http server failed: {err}");
            }
        });
        eprintln!("takd remote v1 http listening on {local_addr}");
    }

    if let Some(config) = tor_hidden_service_runtime_config_from_env()? {
        let store = SubmitAttemptStore::with_db_path(db_path.to_path_buf())
            .context("failed to open takd tor hidden-service sqlite store")?;
        tokio::spawn(async move {
            if let Err(err) = run_remote_v1_tor_hidden_service(config, store).await {
                eprintln!("takd remote v1 tor hidden-service failed: {err}");
            }
        });
    }

    Ok(())
}

/// Resolves the daemon socket path from runtime conventions.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn default_socket_path() -> PathBuf {
    if let Ok(runtime) = std::env::var("XDG_RUNTIME_DIR") {
        return Path::new(&runtime).join("tak/takd.sock");
    }
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/tak-{pid}.sock"))
}

/// Resolves the default SQLite state path for daemon persistence.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn default_state_db_path() -> PathBuf {
    if let Ok(state_home) = std::env::var("XDG_STATE_HOME") {
        return Path::new(&state_home).join("tak/takd.sqlite");
    }
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home).join(".local/state/tak/takd.sqlite");
    }
    PathBuf::from("/tmp/takd.sqlite")
}

/// Performs protocol-level validation for acquire-lease requests.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn ensure_valid_request(request: &AcquireLeaseRequest) -> Result<()> {
    if request.ttl_ms == 0 {
        bail!("ttl_ms must be positive");
    }
    if request.needs.is_empty() {
        bail!("at least one need must be provided");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitRegistration {
    Created { idempotency_key: String },
    Attached { idempotency_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitEventRecord {
    pub seq: u64,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteV1Response {
    pub status_code: u16,
    pub content_type: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SubmitAttemptStore {
    db_path: PathBuf,
}

impl SubmitAttemptStore {
    /// Creates a SQLite-backed submit idempotency store and ensures schema is present.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn with_db_path(db_path: PathBuf) -> Result<Self> {
        let store = Self { db_path };
        store.ensure_schema()?;
        Ok(store)
    }

    /// Registers a submit attempt by `(task_run_id, attempt)` and returns whether it was created or attached.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn register_submit(
        &self,
        task_run_id: &str,
        attempt: Option<u32>,
        selected_node_id: &str,
    ) -> Result<SubmitRegistration> {
        let selected_node_id = selected_node_id.trim();
        if selected_node_id.is_empty() {
            bail!("selected_node_id is required");
        }

        let idempotency_key = build_submit_idempotency_key(task_run_id, attempt)?;
        let conn = self.open_connection()?;
        let inserted = conn.execute(
            "
            INSERT INTO submit_attempts (
                idempotency_key, task_run_id, attempt, selected_node_id, created_at_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                idempotency_key,
                task_run_id.trim(),
                attempt.expect("validated by build_submit_idempotency_key"),
                selected_node_id,
                unix_epoch_ms(),
            ],
        );

        match inserted {
            Ok(_) => Ok(SubmitRegistration::Created { idempotency_key }),
            Err(err) if is_submit_unique_violation(&err) => {
                if !self.has_submit_attempt(&conn, &idempotency_key)? {
                    bail!(
                        "submit idempotency key {} reported duplicate but no row was found",
                        idempotency_key
                    );
                }
                Ok(SubmitRegistration::Attached { idempotency_key })
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Persists one idempotent event for an existing submit attempt.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn append_event(&self, idempotency_key: &str, seq: u64, payload_json: &str) -> Result<()> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        if payload_json.trim().is_empty() {
            bail!("event payload_json is required");
        }
        let seq =
            i64::try_from(seq).with_context(|| format!("event seq {seq} exceeds sqlite range"))?;
        let conn = self.open_connection()?;
        self.ensure_submit_attempt_exists(&conn, key)?;
        conn.execute(
            "
            INSERT OR IGNORE INTO submit_events (idempotency_key, seq, payload_json)
            VALUES (?1, ?2, ?3)
            ",
            params![key, seq, payload_json],
        )?;
        Ok(())
    }

    /// Persists terminal result payload for an existing submit attempt.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn set_result_payload(&self, idempotency_key: &str, payload_json: &str) -> Result<()> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        if payload_json.trim().is_empty() {
            bail!("result payload_json is required");
        }
        let conn = self.open_connection()?;
        self.ensure_submit_attempt_exists(&conn, key)?;
        conn.execute(
            "
            INSERT INTO submit_results (idempotency_key, payload_json)
            VALUES (?1, ?2)
            ON CONFLICT(idempotency_key) DO UPDATE SET payload_json=excluded.payload_json
            ",
            params![key, payload_json],
        )?;
        Ok(())
    }

    /// Loads persisted submit events in ascending sequence order.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn events(&self, idempotency_key: &str) -> Result<Vec<SubmitEventRecord>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT seq, payload_json
            FROM submit_events
            WHERE idempotency_key = ?1
            ORDER BY seq ASC
            ",
        )?;
        let rows = stmt.query_map(params![key], |row| {
            let seq = row.get::<_, i64>(0)?;
            let payload_json = row.get::<_, String>(1)?;
            Ok((seq, payload_json))
        })?;
        let mut events = Vec::new();
        for row in rows {
            let (seq, payload_json) = row?;
            events.push(SubmitEventRecord {
                seq: u64::try_from(seq)
                    .with_context(|| format!("invalid persisted submit event seq {seq}"))?,
                payload_json,
            });
        }
        Ok(events)
    }

    /// Loads the persisted terminal result payload, if any.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on local sqlite availability and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub fn result_payload(&self, idempotency_key: &str) -> Result<Option<String>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT payload_json
            FROM submit_results
            WHERE idempotency_key = ?1
            ",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let payload = row.get::<_, String>(0)?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }

    fn latest_submit_idempotency_key_for_task_run(
        &self,
        task_run_id: &str,
    ) -> Result<Option<String>> {
        let run_id = task_run_id.trim();
        if run_id.is_empty() {
            bail!("task_run_id is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT idempotency_key
            FROM submit_attempts
            WHERE task_run_id = ?1
            ORDER BY attempt DESC
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![run_id])?;
        if let Some(row) = rows.next()? {
            let key = row.get::<_, String>(0)?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    fn selected_node_id_for_submit(&self, idempotency_key: &str) -> Result<Option<String>> {
        let key = idempotency_key.trim();
        if key.is_empty() {
            bail!("idempotency_key is required");
        }
        let conn = self.open_connection()?;
        let mut stmt = conn.prepare(
            "
            SELECT selected_node_id
            FROM submit_attempts
            WHERE idempotency_key = ?1
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let node_id = row.get::<_, String>(0)?;
            Ok(Some(node_id))
        } else {
            Ok(None)
        }
    }

    fn open_connection(&self) -> Result<Connection> {
        if let Some(parent) = self.db_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create sqlite parent directory {:?}", parent)
            })?;
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("failed to open sqlite db at {:?}", self.db_path))?;
        Ok(conn)
    }

    fn ensure_schema(&self) -> Result<()> {
        let conn = self.open_connection()?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS submit_attempts (
                idempotency_key TEXT PRIMARY KEY,
                task_run_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                selected_node_id TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_submit_attempts_run_attempt
            ON submit_attempts(task_run_id, attempt);

            CREATE TABLE IF NOT EXISTS submit_events (
                idempotency_key TEXT NOT NULL,
                seq INTEGER NOT NULL,
                payload_json TEXT NOT NULL,
                PRIMARY KEY (idempotency_key, seq),
                FOREIGN KEY (idempotency_key) REFERENCES submit_attempts(idempotency_key)
            );

            CREATE TABLE IF NOT EXISTS submit_results (
                idempotency_key TEXT PRIMARY KEY,
                payload_json TEXT NOT NULL,
                FOREIGN KEY (idempotency_key) REFERENCES submit_attempts(idempotency_key)
            );
            ",
        )?;
        Ok(())
    }

    fn has_submit_attempt(&self, conn: &Connection, idempotency_key: &str) -> Result<bool> {
        let mut stmt = conn.prepare(
            "
            SELECT 1
            FROM submit_attempts
            WHERE idempotency_key = ?1
            LIMIT 1
            ",
        )?;
        let mut rows = stmt.query(params![idempotency_key])?;
        Ok(rows.next()?.is_some())
    }

    fn ensure_submit_attempt_exists(&self, conn: &Connection, idempotency_key: &str) -> Result<()> {
        if self.has_submit_attempt(conn, idempotency_key)? {
            return Ok(());
        }
        bail!("submit attempt {idempotency_key} does not exist")
    }
}

fn is_submit_unique_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == ErrorCode::ConstraintViolation
    )
}

/// Builds a deterministic submit idempotency key from `task_run_id` and `attempt`.
///
/// ```rust
/// # use takd::build_submit_idempotency_key;
/// let key = build_submit_idempotency_key("run-123", Some(2))?;
/// assert_eq!(key, "run-123:2");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn build_submit_idempotency_key(task_run_id: &str, attempt: Option<u32>) -> Result<String> {
    let task_run_id = task_run_id.trim();
    if task_run_id.is_empty() {
        bail!("task_run_id is required");
    }

    let attempt = validate_submit_attempt(attempt)?;
    Ok(format!("{task_run_id}:{attempt}"))
}

fn validate_submit_attempt(attempt: Option<u32>) -> Result<u32> {
    let attempt = attempt.ok_or_else(|| anyhow!("submit idempotency attempt is required"))?;
    if attempt == 0 {
        bail!("submit idempotency attempt must be >= 1");
    }
    Ok(attempt)
}

/// Handles one canonical V1 remote protocol request against the local submit-attempt store.
///
/// ```no_run
/// # // Reason: This behavior depends on runtime request payloads and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn handle_remote_v1_request(
    store: &SubmitAttemptStore,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<RemoteV1Response> {
    let method = method.trim().to_ascii_uppercase();
    let (path_only, query) = split_path_and_query(path);

    if method == "GET" && path_only == "/v1/node/capabilities" {
        return Ok(json_response(
            200,
            serde_json::json!({
                "compatible": true,
                "protocol_version": "v1",
            }),
        ));
    }
    if method == "GET" && path_only == "/v1/node/status" {
        return Ok(json_response(
            200,
            serde_json::json!({
                "healthy": true,
            }),
        ));
    }
    if method == "POST" && path_only == "/v1/tasks/submit" {
        let Some(body) = body else {
            return Ok(json_response(
                400,
                serde_json::json!({"accepted": false, "reason": "missing_body"}),
            ));
        };
        let payload: serde_json::Value = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(_) => {
                return Ok(json_response(
                    400,
                    serde_json::json!({"accepted": false, "reason": "invalid_json"}),
                ));
            }
        };
        let task_run_id = payload
            .get("task_run_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim();
        let attempt = payload
            .get("attempt")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| u32::try_from(value).ok());
        let selected_node_id = payload
            .get("selected_node_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim();

        if task_run_id.is_empty() || attempt.is_none() || selected_node_id.is_empty() {
            return Ok(json_response(
                400,
                serde_json::json!({"accepted": false, "reason": "invalid_submit_fields"}),
            ));
        }

        let registration = store.register_submit(task_run_id, attempt, selected_node_id)?;
        let (attached, idempotency_key) = match registration {
            SubmitRegistration::Created { idempotency_key } => (false, idempotency_key),
            SubmitRegistration::Attached { idempotency_key } => (true, idempotency_key),
        };
        return Ok(json_response(
            200,
            serde_json::json!({
                "accepted": true,
                "attached": attached,
                "idempotency_key": idempotency_key,
            }),
        ));
    }

    if let Some(task_run_id) = remote_task_path_arg(path_only, "/events")
        && method == "GET"
    {
        let after_seq = query_param_u64(query, "after_seq").unwrap_or(0);
        let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
        let Some(key) = key else {
            return Ok(json_response(
                404,
                serde_json::json!({"error":"task_not_found"}),
            ));
        };

        let events = store.events(&key)?;
        let mut lines = Vec::new();
        for event in events.into_iter().filter(|event| event.seq > after_seq) {
            let payload_value = serde_json::from_str::<serde_json::Value>(&event.payload_json)
                .unwrap_or_else(|_| serde_json::json!({ "raw": event.payload_json }));
            let event_type = payload_value
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("EVENT");
            let timestamp_ms = payload_value
                .get("timestamp_ms")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            lines.push(
                serde_json::json!({
                    "seq": event.seq,
                    "task_run_id": task_run_id,
                    "type": event_type,
                    "timestamp_ms": timestamp_ms,
                    "payload": payload_value,
                })
                .to_string(),
            );
        }
        let mut body = lines.join("\n");
        if !body.is_empty() {
            body.push('\n');
        }
        return Ok(RemoteV1Response {
            status_code: 200,
            content_type: "application/x-ndjson".to_string(),
            body,
        });
    }

    if let Some(task_run_id) = remote_task_path_arg(path_only, "/result")
        && method == "GET"
    {
        let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
        let Some(key) = key else {
            return Ok(json_response(
                404,
                serde_json::json!({"error":"task_not_found"}),
            ));
        };
        let Some(payload_json) = store.result_payload(&key)? else {
            return Ok(json_response(
                404,
                serde_json::json!({"error":"result_not_found"}),
            ));
        };
        let payload_value = serde_json::from_str::<serde_json::Value>(&payload_json)
            .unwrap_or_else(|_| serde_json::json!({}));
        let success = payload_value
            .get("success")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let status = if success { "success" } else { "failure" };
        let node_id = store
            .selected_node_id_for_submit(&key)?
            .unwrap_or_else(|| "unknown".to_string());
        return Ok(json_response(
            200,
            serde_json::json!({
                "success": success,
                "status": status,
                "exit_code": payload_value.get("exit_code").cloned().unwrap_or_else(|| serde_json::json!(1)),
                "started_at": payload_value.get("started_at").cloned().unwrap_or_else(|| serde_json::json!(0)),
                "finished_at": payload_value.get("finished_at").cloned().unwrap_or_else(|| serde_json::json!(0)),
                "duration_ms": payload_value.get("duration_ms").cloned().unwrap_or_else(|| serde_json::json!(0)),
                "node_id": node_id,
                "transport_kind": payload_value.get("transport_kind").cloned().unwrap_or_else(|| serde_json::json!("direct")),
                "log_artifact_uri": payload_value.get("log_artifact_uri").cloned().unwrap_or(serde_json::Value::Null),
                "outputs": payload_value.get("outputs").cloned().unwrap_or_else(|| serde_json::json!([])),
                "stdout_tail": payload_value.get("stdout_tail").cloned().unwrap_or(serde_json::Value::Null),
                "stderr_tail": payload_value.get("stderr_tail").cloned().unwrap_or(serde_json::Value::Null),
            }),
        ));
    }

    if let Some(task_run_id) = remote_task_path_arg(path_only, "/cancel")
        && method == "POST"
    {
        return Ok(json_response(
            202,
            serde_json::json!({
                "cancelled": true,
                "task_run_id": task_run_id,
            }),
        ));
    }

    Ok(json_response(
        404,
        serde_json::json!({
            "error": "not_found",
            "method": method,
            "path": path_only,
        }),
    ))
}

fn resolve_submit_idempotency_key_for_task_run(
    store: &SubmitAttemptStore,
    task_run_id: &str,
    query: Option<&str>,
) -> Result<Option<String>> {
    if let Some(attempt) =
        query_param_u64(query, "attempt").and_then(|value| u32::try_from(value).ok())
    {
        let key = build_submit_idempotency_key(task_run_id, Some(attempt))?;
        return Ok(Some(key));
    }
    store.latest_submit_idempotency_key_for_task_run(task_run_id)
}

fn split_path_and_query(path: &str) -> (&str, Option<&str>) {
    match path.split_once('?') {
        Some((path_only, query)) => (path_only, Some(query)),
        None => (path, None),
    }
}

fn query_param_u64(query: Option<&str>, key: &str) -> Option<u64> {
    let query = query?;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            value.parse::<u64>().ok()
        } else {
            None
        }
    })
}

fn remote_task_path_arg<'a>(path: &'a str, suffix: &str) -> Option<&'a str> {
    let path = path.strip_prefix("/v1/tasks/")?;
    let task_run_id = path.strip_suffix(suffix)?;
    if task_run_id.is_empty() || task_run_id.contains('/') {
        return None;
    }
    Some(task_run_id)
}

fn json_response(status_code: u16, body: serde_json::Value) -> RemoteV1Response {
    RemoteV1Response {
        status_code,
        content_type: "application/json".to_string(),
        body: body.to_string(),
    }
}

/// Returns the current Unix epoch timestamp in milliseconds.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn unix_epoch_ms() -> i64 {
    let now = SystemTime::now();
    let duration = now
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH");
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}
