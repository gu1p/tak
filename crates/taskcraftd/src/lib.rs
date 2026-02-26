//! Taskcraft daemon protocol and lease coordination engine.
//!
//! The daemon serves NDJSON requests over a Unix socket and coordinates machine-wide
//! limiter leases with optional SQLite-backed persistence and history.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use taskcraft_core::model::Scope;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
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
                ttl_ms: row.get::<_, u64>(6)?,
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
                    ttl_ms: row.ttl_ms,
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
        let expires_at_ms = unix_epoch_ms() + record.ttl_ms as i64;
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
                record.ttl_ms,
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
    ttl_ms: u64,
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
    let db_path = std::env::var("TASKCRAFTD_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_state_db_path());
    let manager = new_shared_manager_with_db(db_path)?;

    {
        let mut guard = manager
            .lock()
            .map_err(|_| anyhow!("lease manager lock poisoned"))?;
        guard.set_capacity("cpu", Scope::Machine, None, 8.0);
        guard.set_capacity("ram_gib", Scope::Machine, None, 32.0);
    }

    run_server(socket_path, manager).await
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
        return Path::new(&runtime).join("taskcraft/taskcraftd.sock");
    }
    let pid = std::process::id();
    PathBuf::from(format!("/tmp/taskcraft-{pid}.sock"))
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
        return Path::new(&state_home).join("taskcraft/taskcraftd.sqlite");
    }
    if let Ok(home) = std::env::var("HOME") {
        return Path::new(&home).join(".local/state/taskcraft/taskcraftd.sqlite");
    }
    PathBuf::from("/tmp/taskcraftd.sqlite")
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
