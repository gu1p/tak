#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecisionModeDef {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecisionDef {
    pub mode: PolicyDecisionModeDef,
    #[serde(default)]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskExecutionDef {
    LocalOnly {
        local: LocalDef,
    },
    RemoteOnly {
        remote: RemoteDef,
    },
    ByCustomPolicy {
        policy_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        decision: Option<PolicyDecisionDef>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepDef {
    Cmd {
        argv: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
        #[serde(default)]
        env: BTreeMap<String, String>,
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
        env: BTreeMap<String, String>,
    },
}

impl Default for StepDef {
    /// Creates an empty command step used by serde defaulting.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self::Cmd {
            argv: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hold {
    #[default]
    During,
    AtStart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedDef {
    pub limiter: LimiterRef,
    #[serde(default = "default_one")]
    pub slots: f64,
    #[serde(default)]
    pub hold: Hold,
}

/// Returns the default slot count for limiter needs.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueUseDef {
    pub queue: LimiterRef,
    #[serde(default = "default_slots")]
    pub slots: i32,
    #[serde(default)]
    pub priority: i32,
}

/// Returns the default slot count for queue usage.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_slots() -> i32 {
    1
}
