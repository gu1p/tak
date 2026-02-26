# PLAN.md — Taskcraft: Multi‑User Task Orchestrator (Rust) with Monty‑Evaluated `TASKS.py`

> **What this is**: a build/task *orchestrator* (not a build system) that runs your existing tooling (`cargo`, `pnpm`, `uv`, scripts, etc.) but coordinates **who can run what, when**, across **many users** and **many worktrees/clones** on the **same machine**.

---

## 0) Executive summary

- **Definitions** live in `TASKS.py` files scattered through the repo tree.
- `TASKS.py` is evaluated by **Monty** (a sandboxed subset of Python embedded in Rust) and must return **pure data**: tasks, limiters, queues, defaults. (Read about it here https://github.com/pydantic/monty )
- The Rust engine:
  - recursively discovers `TASKS.py` respecting `.gitignore`
  - evaluates them using Monty with a tiny DSL (pure function constructors)
  - merges definitions, resolves labels, builds a DAG
  - executes tasks locally, but uses a **machine-wide daemon** for **locks, queues, resource caps, rate limits, process caps**.

---

## 1) Goals / Non‑Goals

### Goals (v1)
1. Pleasant, declarative task definitions (Python-like).
2. Recursive discovery of tasks per directory (namespaced).
3. DAG execution with dependency ordering and parallelism where possible.
4. **Machine-wide coordination** between users and independent invocations:
   - global locks (e.g., UI monitor)
   - resource pools (CPU/RAM “slots”)
   - queues with caps and priorities
   - process caps (“max 2 emulators running”)
   - rate limits (“max N starts per second”)
5. Safe task definitions: `TASKS.py` cannot touch host filesystem/env/network.
6. Good UX: list/explain/graph/status.

### Non‑Goals (v1)
- Replace toolchains (no special wrappers for cargo/pnpm/uv beyond `cmd(...)`).
- Hermetic builds, remote caching, distributed execution.
- Full Python runtime for `TASKS.py` (must fit Monty subset).

---

## 2) Architecture

### Components
1. **CLI (`taskcraft`)**
   - discovers + loads tasks from the repo
   - builds DAG
   - schedules runnable nodes locally (deps-aware)
   - requests leases from daemon before executing a task

2. **Daemon (`taskcraftd`)**
   - machine-wide coordinator (single decision point)
   - maintains limiter state + queues + leases
   - does **atomic multi-limiter acquisition** (deadlock avoidance)
   - tracks lease TTL + renewal (heartbeat)

3. **Shared state storage**
   - SQLite (recommended) for:
     - active leases
     - queue backlog
     - history/audit (who held `ui_monitor` when)
   - plus in-memory fast path

### Why a daemon?
You want:
- fairness/priority queues
- cross-user coordination
- atomic acquisition of *multiple* resources

That is much simpler with a machine-wide daemon than with ad-hoc lockfiles.

---

## 3) Task discovery

### Root detection
Workspace root (priority order):
1. `taskcraft.toml` marker
2. Git root (`.git`)
3. fallback: current directory (warn)

### Discovery rules
- Find all `TASKS.py` under root recursively.
- Respect ignore rules:
  - `.gitignore`, `.git/info/exclude`, global gitignore, etc.
- Recommended Rust library: the `ignore` crate’s `WalkBuilder`, which implements gitignore-aware walking.

### Namespacing
- Package namespace is directory path from root: `//apps/web`
- `apps/web/TASKS.py` task `build` => label `//apps/web:build`

---

## 4) The `TASKS.py` contract (Monty)

### Key design constraints (Monty)
- Monty runs a **subset of Python**
- It can be sandboxed to block filesystem/env/network access
- It supports external functions **only if you expose them**
- It supports a type-checking flow (via its bundled type-checking crate)
- Rust API supports compiling/running code quickly and serializing compiled runs

### Contract
Each `TASKS.py` must evaluate to a single dict:

```python
{
  "spec_version": 1,
  "tasks":   [ ...TaskDef... ],
  "limiters":[ ...LimiterDef... ],   # optional
  "queues":  [ ...QueueDef... ],     # optional
  "exclude": [ ...patterns... ],     # optional
  "defaults":{ ... },                # optional
}
```

**Hard rule**: `TASKS.py` is **definition only**. No host access.

### How the DSL is provided
You do **not** `import taskcraft` from `TASKS.py`. Instead, the engine prepends a **prelude** string that defines the DSL functions/constants.

Example load flow per file:

1. `code = PRELUDE + "\n\n" + read_to_string("TASKS.py")`
2. (optional) `type_check(code, stubs=DSL_STUBS)`
3. compile + execute in Monty
4. take the final expression result as the module spec

---

## 5) DSL (Monty-friendly)

### Constants
```python
MACHINE  = "machine"
USER     = "user"
PROJECT  = "project"
WORKTREE = "worktree"

DURING   = "during"
AT_START = "at_start"

FIFO     = "fifo"
PRIORITY = "priority"
```

### Constructors (pure functions returning dict/list)
Minimum set:

- `module_spec(tasks, limiters=None, queues=None, exclude=None, defaults=None)`
- `task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, tags=None, doc=None)`
- `cmd(*argv, cwd=None, env=None)`
- `script(path, *argv, interpreter=None, cwd=None, env=None)`
- `need(name, slots=1, scope=PROJECT, hold=DURING)`
- `queue_use(name, scope=MACHINE, slots=1, priority=0)`
- limiter defs:
  - `resource(name, capacity, unit=None, scope=MACHINE)`
  - `lock(name, scope=MACHINE)`
  - `queue_def(name, slots, discipline=FIFO, max_pending=None, scope=MACHINE)`
  - `rate_limit(name, burst, refill_per_second, scope=MACHINE)`
  - `process_cap(name, max_running, match=None, scope=MACHINE)`
- retry defs:
  - `retry(attempts=1, on_exit=None, backoff=None)`
  - `fixed(seconds)`
  - `exp_jitter(min_s=1, max_s=60, jitter="full")`

### Example `TASKS.py`
```python
SPEC = module_spec(
  tasks=[
    task(
      "build",
      steps=[cmd("pnpm", "build")],
      needs=[need("cpu", 4, scope=MACHINE), need("ram_gib", 4, scope=MACHINE)],
      tags=["build","web"],
    ),
    task(
      "test_ui",
      deps=[":build"],
      steps=[cmd("pnpm","test:ui")],
      needs=[
        need("ui_monitor", 1, scope=MACHINE),
        need("cpu", 4, scope=MACHINE),
        need("ram_gib", 8, scope=MACHINE),
        need("chrome", 1, scope=MACHINE),
      ],
      queue=queue_use("ui", scope=MACHINE, slots=1, priority=50),
      retry=retry(attempts=2, on_exit=[1], backoff=exp_jitter(min_s=2, max_s=60)),
      tags=["ui","playwright"],
    )
  ]
)
SPEC
```

---

## 6) Concurrency model

Everything reduces to **slots**:

- **Lock**: capacity = 1 slot
- **Resource pool**: capacity = N slots (e.g., CPU “cores”, RAM “GiB”)
- **Queue**: also has slots, plus ordering/priority
- **Rate limit**: token bucket (burst + refill rate); tasks acquire tokens with `hold=AT_START`
- **Process cap**: capacity dynamically computed by `max_running - observed_running`

### Scopes
All limiters have a scope:
- `machine`: shared across all users + repos
- `user`: shared per OS user
- `project`: shared across clones/worktrees of the same “project id”
- `worktree`: shared within a single checkout path

### Deadlock avoidance
Daemon MUST support **atomic acquisition**:
- client requests a set of needs in one request
- daemon grants **all** or **none**
- daemon queues the request (with priority) if it can’t satisfy it

### Leases (TTL)
Leases are time-bound:
- daemon issues `lease_id` with TTL (e.g., 30s)
- client renews periodically (e.g., every 10s)
- if client dies, lease expires and resources are reclaimed

---

## 7) Project identity

For `scope="project"` you need a stable project key across clones/worktrees.

### Rule
- If `taskcraft.toml` contains `project_id = "..."`, use it.
- Else attempt to derive:
  - hash of git remote origin URL + repo name (best effort)
- Strongly recommend explicit `project_id` to avoid surprises.

---

## 8) Daemon protocol (Unix socket, JSON frames)

### Message framing
- newline-delimited JSON (NDJSON) OR length-prefixed frames
- each request includes `request_id`

### Core requests
1. `AcquireLease`
2. `RenewLease`
3. `ReleaseLease`
4. `Status`

#### AcquireLease request
```json
{
  "type": "AcquireLease",
  "request_id": "uuid",
  "client": {"user":"alice","pid":1234,"session_id":"..."},
  "task": {"label":"//apps/web:test_ui","attempt":1},
  "queue": {"name":"ui","scope":"machine","slots":1,"priority":50},
  "needs": [
    {"limiter":{"name":"ui_monitor","scope":"machine"},"slots":1,"hold":"during"},
    {"limiter":{"name":"cpu","scope":"machine"},"slots":4,"hold":"during"}
  ],
  "ttl_ms": 30000
}
```

#### AcquireLease response (granted)
```json
{
  "type":"LeaseGranted",
  "request_id":"uuid",
  "lease":{"lease_id":"uuid","ttl_ms":30000,"renew_after_ms":10000}
}
```

#### AcquireLease response (pending)
```json
{
  "type":"LeasePending",
  "request_id":"uuid",
  "pending":{"queue_position": 7}
}
```

---

## 9) Rust crate layout (recommended)

- `crates/taskcraft-core`
  - data model (serde types)
  - label parsing/resolution
  - validation

- `crates/taskcraft-loader`
  - discovery walker (gitignore aware)
  - monty evaluation + type checking
  - merge module specs into workspace spec

- `crates/taskcraft-exec`
  - task runner (spawn processes, logs, retries)
  - client for daemon protocol

- `crates/taskcraftd`
  - daemon server, limiter bookkeeping, lease/queue management

- `crates/taskcraft` (binary)
  - CLI UX + wiring

---

## 10) Rust data models (serde) — **copy/paste skeleton**

> These models are the canonical “wire format” between Monty output (after validation) and the engine.
> The Monty DSL should output JSON-compatible types only; we validate and then deserialize.

### 10.1 Common types: labels, scopes, limiter refs

```rust
// crates/taskcraft-core/src/model.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Machine,
    User,
    Project,
    Worktree,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LimiterRef {
    pub name: String,
    pub scope: Scope,
    // scope_key is resolved by the loader, not authored in TASKS.py
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskLabel {
    pub package: String, // e.g. "//apps/web"
    pub name: String,    // e.g. "build"
}

impl std::fmt::Display for TaskLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package, self.name)
    }
}
```

### 10.2 ModuleSpec

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub spec_version: u32,
    #[serde(default)]
    pub tasks: Vec<TaskDef>,
    #[serde(default)]
    pub limiters: Vec<LimiterDef>,
    #[serde(default)]
    pub queues: Vec<QueueDef>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<QueueUseDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryDef>,
    #[serde(default)]
    pub tags: Vec<String>,
}
```

### 10.3 Tasks, steps, needs, queue use

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDef {
    pub name: String,
    #[serde(default)]
    pub doc: String,
    #[serde(default)]
    pub deps: Vec<String>, // raw labels; resolved later using package context
    #[serde(default)]
    pub steps: Vec<StepDef>,
    #[serde(default)]
    pub needs: Vec<NeedDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<QueueUseDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_s: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepDef {
    Cmd {
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    },
    Script {
        path: String,
        #[serde(default)]
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interpreter: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default)]
        env: std::collections::BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    During,
    AtStart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedDef {
    pub limiter: LimiterRef,
    #[serde(default = "default_one")]
    pub slots: f64,
    #[serde(default = "default_hold")]
    pub hold: Hold,
}

fn default_one() -> f64 { 1.0 }
fn default_hold() -> Hold { Hold::During }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueUseDef {
    pub queue: LimiterRef, // refers to a queue by name/scope
    #[serde(default = "default_i32_one")]
    pub slots: i32,
    #[serde(default)]
    pub priority: i32,
}

fn default_i32_one() -> i32 { 1 }
```

### 10.4 Limiters and queues

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LimiterDef {
    Resource {
        name: String,
        scope: Scope,
        capacity: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },
    Lock {
        name: String,
        scope: Scope,
    },
    RateLimit {
        name: String,
        scope: Scope,
        burst: u32,
        refill_per_second: f64,
    },
    ProcessCap {
        name: String,
        scope: Scope,
        max_running: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#match: Option<String>, // regex/substring
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDef {
    pub name: String,
    pub scope: Scope,
    pub slots: u32,
    #[serde(default = "default_fifo")]
    pub discipline: QueueDiscipline,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pending: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueDiscipline {
    Fifo,
    Priority,
}

fn default_fifo() -> QueueDiscipline { QueueDiscipline::Fifo }
```

### 10.5 Workspace registry (post-merge, post-resolution)

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct WorkspaceSpec {
    pub project_id: String,
    pub root: std::path::PathBuf,
    pub tasks: BTreeMap<TaskLabel, ResolvedTask>,
    pub limiters: BTreeMap<LimiterKey, ResolvedLimiter>,
    pub queues: BTreeMap<LimiterKey, ResolvedQueue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LimiterKey {
    pub scope: Scope,
    pub scope_key: Option<String>, // e.g. project_id for Project scope
    pub name: String,
}

// After label resolution and applying defaults:
#[derive(Debug, Clone)]
pub struct ResolvedTask {
    pub label: TaskLabel,
    pub doc: String,
    pub deps: Vec<TaskLabel>,
    pub steps: Vec<StepDef>,
    pub needs: Vec<NeedDef>,
    pub queue: Option<QueueUseDef>,
    pub retry: RetryDef,
    pub timeout_s: Option<u64>,
    pub tags: Vec<String>,
}
```

### 10.6 Retry model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryDef {
    #[serde(default = "default_attempts")]
    pub attempts: u32,
    #[serde(default)]
    pub on_exit: Vec<i32>,
    #[serde(default)]
    pub backoff: BackoffDef,
}

fn default_attempts() -> u32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackoffDef {
    Fixed { seconds: f64 },
    ExpJitter { min_s: f64, max_s: f64, #[serde(default = "default_jitter")] jitter: String },
}

impl Default for BackoffDef {
    fn default() -> Self { BackoffDef::Fixed { seconds: 0.0 } }
}

fn default_jitter() -> String { "full".to_string() }
```

---

## 11) Monty integration details (Rust)

### 11.1 Compile + run in Monty
Monty’s README shows a Rust flow using `MontyRun` and `MontyObject`:

```rust
use monty::{MontyRun, MontyObject, NoLimitTracker, PrintWriter};

let code = r#"
def fib(n):
    if n <= 1:
        return n
    return fib(n-1) + fib(n-2)
fib(x)
"#;

let runner = MontyRun::new(
    code.to_owned(),
    "fib.py",
    vec!["x".to_owned()],
    vec![], // external function names
).unwrap();

let result = runner.run(
    vec![MontyObject::Int(10)],
    NoLimitTracker,
    &mut PrintWriter::Stdout,
).unwrap();
```

**Your usage** for `TASKS.py`:
- `inputs = vec![]`
- `external_functions = vec![]`
- enforce resource limits (see next section)
- `script_name = "<relative path>/TASKS.py"` for good diagnostics
- code = `PRELUDE + user_code`

### 11.2 Resource limits
Prefer `LimitedTracker` (instead of `NoLimitTracker`) so malicious or accidental runaway definitions can’t hang the loader.

Use `monty::ResourceLimits` + `monty::LimitedTracker`.

### 11.3 Type checking for a better author experience
Monty exposes a type checker via the `monty-type-checking` crate, re-exporting:

- `SourceFile`
- `type_check(...) -> Result<Option<TypeCheckingDiagnostics>, String>`

This works entirely in-memory and can apply a stub file by injecting `from <stub> import *` at the top.

Minimal flow:

```rust
use monty_type_checking::{SourceFile, type_check};

let src = SourceFile::new(&code, "TASKS.py");
let stubs = SourceFile::new(DSL_STUBS, "taskcraft_dsl.pyi");

match type_check(&src, Some(&stubs))? {
    None => {} // ok
    Some(diags) => {
        // show diags to user and fail load
        eprintln!("{diags}");
        anyhow::bail!("Type errors in TASKS.py");
    }
}
```

### 11.4 Converting Monty output to JSON
`MontyRun::run` returns `MontyObject`. For safety, v1 requires `TASKS.py` output be **JSON-compatible**:
- null/bool/int/float/string/list/dict with **string keys**
- no tuples/bytes/sets/etc.

Implement a strict converter:

```rust
use monty::MontyObject;
use serde_json::{Map, Value};

pub fn monty_to_json(obj: MontyObject) -> anyhow::Result<Value> {
    Ok(match obj {
        MontyObject::None => Value::Null,
        MontyObject::Bool(b) => Value::Bool(b),
        MontyObject::Int(i) => Value::Number(i.into()),
        MontyObject::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .ok_or_else(|| anyhow::anyhow!("non-finite float"))?,
        MontyObject::String(s) => Value::String(s),
        MontyObject::List(items) => Value::Array(
            items.into_iter().map(monty_to_json).collect::<Result<_,_>>()?
        ),
        MontyObject::Dict(pairs) => {
            let mut map = Map::new();
            for (k, v) in pairs.into_iter() {
                let key = match k {
                    MontyObject::String(s) => s,
                    _ => anyhow::bail!("dict key must be string, got {k:?}"),
                };
                map.insert(key, monty_to_json(v)?);
            }
            Value::Object(map)
        }
        other => anyhow::bail!("TASKS.py returned non-JSON value: {other:?}"),
    })
}
```

Then `serde_json::from_value::<ModuleSpec>(json_value)`.

### 11.5 Caching compiled runs
Monty supports serializing compiled runs (`MontyRun::dump/load`) for speed:
- cache by file content hash (and Monty version)
- in `.taskcraft/cache/` or global cache

---

## 12) Loader pipeline (end-to-end)

For each discovered `TASKS.py`:

1. Read file
2. Build `code = PRELUDE + user_code`
3. (optional but recommended) `type_check(code, DSL_STUBS)`
4. Compile: `MontyRun::new(code, script_name, inputs=[], externals=[])`
5. Execute: `runner.run([], LimitedTracker, NullWriter)`
6. Convert result: `MontyObject -> serde_json::Value` (strict)
7. Deserialize `ModuleSpec`
8. Apply module-local exclude patterns to discovery (optional)
9. Merge tasks/limiters/queues into workspace registry

After all modules loaded:
10. Resolve labels (`:x` -> `//current_pkg:x`)
11. Apply defaults
12. Validate:
    - no duplicate labels
    - all deps exist
    - limiter refs exist (or allow late-binding for machine policy)
13. Build DAG

---

## 13) Execution pipeline (client)

1. Choose targets: `taskcraft run //apps/web:test_ui`
2. Expand dependencies (DAG)
3. Maintain a local ready queue of runnable tasks (deps satisfied)
4. For each runnable task:
   - request lease from daemon with needs + queue + priority
   - when granted: spawn steps
   - renew lease until completion
   - release lease with status
5. Retry/backoff as specified
6. Summarize results

---

## 14) Daemon core algorithms (v1)

### 14.1 State
- limiter capacities and current usage
- queue backlogs (per queue key)
- active leases: `lease_id -> resources held, expiry, owner`

### 14.2 AcquireLease
- place request into queue discipline:
  - FIFO: append
  - Priority: insert by priority (with aging optional)
- periodically or on resource release:
  - attempt to satisfy earliest/highest requests
  - must allocate **all needs atomically**
  - if satisfied: grant lease and decrement capacities accordingly
  - if not: leave pending

### 14.3 Rate limit limiter
Token bucket per limiter key:
- tokens <= burst
- refill tokens by `refill_per_second * dt`
- `need(hold=AT_START)` consumes tokens at start and does not “hold” during runtime

### 14.4 Process cap limiter
- daemon periodically samples process table
- counts matches
- available = max_running - observed
- treat as dynamic capacity for acquisition

---

## 15) CLI UX (minimum)

- `taskcraft list`
- `taskcraft explain <label>`
- `taskcraft graph <label>` (DOT/JSON)
- `taskcraft run <label...> [-j N] [--keep-going]`
- `taskcraft status` (who holds locks, what’s waiting)
- `taskcraft daemon start|status` (or integrate with systemd)

---

## 16) Test plan (acceptance)

### Loader
- `.gitignore` respected (ignored directories do not load `TASKS.py`)
- Monty sandbox prevents host access (attempts fail)
- Type checking catches obvious DSL mistakes
- Duplicate task labels fail fast with clear diagnostics

### Scheduling / daemon
- `ui_monitor` lock prevents overlap across separate users
- CPU pool never exceeds capacity across multiple concurrent runs
- Priority queue starts higher priority first
- Lease expiry releases resources if client dies
- Rate limit throttles starts

### End-to-end
- Two terminals, two users:
  - run `test_ui` concurrently -> serialized
  - run CPU-heavy tasks -> limited to capacity

---

## 17) Implementation milestones

1. **Core models + label resolver**
2. **Discovery walker + Monty loader**
3. **CLI list/explain/graph**
4. **Local executor (no daemon)**
5. **Daemon + lease protocol**
6. **Client ↔ daemon integration**
7. **Rate limit + process cap**
8. **UX polish: status/history/logs**

---

## 18) References (for implementers)

- Monty README includes Rust API examples (`MontyRun`, `MontyObject`, dump/load) and overall constraints.
- Monty type checking crate exports `type_check` and `SourceFile` (in-memory, optional stubs injection).

(Provide URLs in your project README; this PLAN intentionally avoids hard-linking to keep it portable.)
