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
        r#match: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDef {
    pub name: String,
    pub scope: Scope,
    pub slots: u32,
    #[serde(default)]
    pub discipline: QueueDiscipline,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_pending: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueDiscipline {
    #[default]
    Fifo,
    Priority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryDef {
    #[serde(default = "default_attempts")]
    pub attempts: u32,
    #[serde(default)]
    pub on_exit: Vec<i32>,
    #[serde(default)]
    pub backoff: BackoffDef,
}

impl Default for RetryDef {
    /// Builds the default retry policy: one attempt and no backoff delay.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self {
            attempts: default_attempts(),
            on_exit: Vec::new(),
            backoff: BackoffDef::default(),
        }
    }
}

/// Returns the default retry attempt count.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_attempts() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BackoffDef {
    Fixed {
        seconds: f64,
    },
    ExpJitter {
        min_s: f64,
        max_s: f64,
        #[serde(default = "default_jitter")]
        jitter: String,
    },
}

impl Default for BackoffDef {
    /// Uses a zero-second fixed backoff as the default strategy.
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn default() -> Self {
        Self::Fixed { seconds: 0.0 }
    }
}

/// Returns the default jitter mode for exponential backoff.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn default_jitter() -> String {
    "full".to_string()
}
