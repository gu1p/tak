use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ResolvedRunError;

mod manifest;
mod validation;

use validation::{validate_relative_path, validate_symlink_target};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceEntryType {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceEntry {
    pub path: String,
    pub entry_type: WorkspaceEntryType,
    pub executable: bool,
    pub symlink_target: Option<String>,
    pub size: u64,
    pub content_sha256: String,
}

impl WorkspaceEntry {
    pub fn file(
        path: impl Into<String>,
        executable: bool,
        size: u64,
        content_sha256: &str,
    ) -> Result<Self, ResolvedRunError> {
        let entry = Self {
            path: path.into(),
            entry_type: WorkspaceEntryType::File,
            executable,
            symlink_target: None,
            size,
            content_sha256: content_sha256.to_owned(),
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn directory(path: impl Into<String>) -> Result<Self, ResolvedRunError> {
        let entry = Self {
            path: path.into(),
            entry_type: WorkspaceEntryType::Directory,
            executable: false,
            symlink_target: None,
            size: 0,
            content_sha256: format!("{:x}", Sha256::digest([])),
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn symlink(
        path: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, ResolvedRunError> {
        let target = target.into();
        let entry = Self {
            path: path.into(),
            entry_type: WorkspaceEntryType::Symlink,
            executable: false,
            symlink_target: Some(target.clone()),
            size: target.len() as u64,
            content_sha256: format!("{:x}", Sha256::digest(target.as_bytes())),
        };
        entry.validate()?;
        Ok(entry)
    }

    pub(super) fn validate(&self) -> Result<(), ResolvedRunError> {
        validate_relative_path(&self.path)?;
        validate_digest(&self.content_sha256)?;
        match self.entry_type {
            WorkspaceEntryType::File if self.symlink_target.is_none() => Ok(()),
            WorkspaceEntryType::Directory
                if !self.executable
                    && self.symlink_target.is_none()
                    && self.size == 0
                    && self.content_sha256 == format!("{:x}", Sha256::digest([])) =>
            {
                Ok(())
            }
            WorkspaceEntryType::Symlink if !self.executable => {
                let target = self.symlink_target.as_deref();
                validate_symlink_target(&self.path, target)?;
                let target = target.expect("validated symlink target");
                if self.size != target.len() as u64
                    || self.content_sha256 != format!("{:x}", Sha256::digest(target.as_bytes()))
                {
                    return Err(ResolvedRunError::new("invalid symlink metadata"));
                }
                Ok(())
            }
            _ => Err(ResolvedRunError::new("invalid workspace entry metadata")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceManifest {
    pub fingerprint: String,
    pub entries: Vec<WorkspaceEntry>,
}

impl WorkspaceManifest {
    pub fn new(
        entries: impl IntoIterator<Item = WorkspaceEntry>,
    ) -> Result<Self, ResolvedRunError> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        for entry in &entries {
            entry.validate()?;
        }
        if entries.windows(2).any(|pair| pair[0].path == pair[1].path) {
            return Err(ResolvedRunError::new("duplicate workspace manifest path"));
        }
        manifest::validate(&entries)?;
        let fingerprint = fingerprint_entries(&entries);
        Ok(Self {
            fingerprint,
            entries,
        })
    }

    pub(super) fn validate(&self) -> Result<(), ResolvedRunError> {
        let canonical = Self::new(self.entries.clone())?;
        if canonical.entries != self.entries || canonical.fingerprint != self.fingerprint {
            return Err(ResolvedRunError::new("workspace manifest is not canonical"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDescriptor {
    pub manifest: WorkspaceManifest,
    pub archive_sha256: String,
    pub archive_size: u64,
}

fn fingerprint_entries(entries: &[WorkspaceEntry]) -> String {
    let bytes = serde_json::to_vec(entries).expect("workspace entries are serializable");
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn validate_digest(value: &str) -> Result<(), ResolvedRunError> {
    validation::validate_digest(value)
}
