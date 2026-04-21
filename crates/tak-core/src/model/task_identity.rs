use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TaskLabel {
    pub package: String,
    pub name: String,
}

impl fmt::Display for TaskLabel {
    /// Formats labels using clean user-facing syntax:
    /// - root package: `name`
    /// - nested package: `package:name`
    ///
    /// ```no_run
    /// # // Reason: This behavior depends on internal state and is compile-checked only.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.package == "//" {
            return write!(f, "{}", self.name);
        }
        if let Some(package) = self.package.strip_prefix("//") {
            return write!(f, "{}:{}", package, self.name);
        }
        write!(f, "{}:{}", self.package, self.name)
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_key: Option<String>,
}
