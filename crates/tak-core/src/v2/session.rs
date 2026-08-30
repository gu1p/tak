use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{Execution, OutputSelector, ValidationError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Affinity {
    PreferSameNode { group: String },
    RequireSameNode { group: String },
}

impl Affinity {
    pub fn prefer_same_node(group: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self::PreferSameNode {
            group: validate_group(group.into())?,
        })
    }

    pub fn require_same_node(group: impl Into<String>) -> Result<Self, ValidationError> {
        Ok(Self::RequireSameNode {
            group: validate_group(group.into())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionReuse {
    Workspace,
    Paths { paths: Vec<OutputSelector> },
    SharedWorkspace { max_parallel_tasks: NonZeroU32 },
    Container,
}

impl SessionReuse {
    pub fn shared_workspace(max_parallel_tasks: u32) -> Result<Self, ValidationError> {
        let max_parallel_tasks =
            NonZeroU32::new(max_parallel_tasks).ok_or(ValidationError::InvalidSharedParallelism)?;
        Ok(Self::SharedWorkspace { max_parallel_tasks })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,
    pub reuse: SessionReuse,
    pub affinity: Option<Affinity>,
    pub execution: Option<Box<Execution>>,
}

impl Session {
    pub fn new(
        name: impl Into<String>,
        reuse: SessionReuse,
        affinity: Option<Affinity>,
    ) -> Result<Self, ValidationError> {
        let name = name.into();
        validate_shared_affinity(&reuse, affinity.as_ref())?;
        Ok(Self {
            id: name.clone(),
            name: Some(name),
            reuse,
            affinity,
            execution: None,
        })
    }

    pub fn effective_affinity(
        &self,
        task_affinity: Option<&Affinity>,
    ) -> Result<Option<Affinity>, ValidationError> {
        if matches!(self.reuse, SessionReuse::SharedWorkspace { .. })
            && task_affinity.is_some_and(|affinity| Some(affinity) != self.affinity.as_ref())
        {
            return Err(ValidationError::SharedWorkspaceAffinityOverride);
        }
        Ok(task_affinity.cloned().or_else(|| self.affinity.clone()))
    }
}

fn validate_group(group: String) -> Result<String, ValidationError> {
    if group.trim().is_empty() {
        return Err(ValidationError::EmptyAffinityGroup);
    }
    Ok(group)
}

fn validate_shared_affinity(
    reuse: &SessionReuse,
    affinity: Option<&Affinity>,
) -> Result<(), ValidationError> {
    if matches!(reuse, SessionReuse::SharedWorkspace { .. })
        && !matches!(affinity, Some(Affinity::RequireSameNode { .. }))
    {
        return Err(ValidationError::SharedWorkspaceRequiresHardAffinity);
    }
    Ok(())
}
