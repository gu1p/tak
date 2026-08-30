use std::collections::BTreeMap;
use std::path::Path;

use super::{WorkspaceEntry, WorkspaceEntryType};
use crate::v2::ResolvedRunError;

pub(super) fn validate(entries: &[WorkspaceEntry]) -> Result<(), ResolvedRunError> {
    let by_path = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for entry in entries {
        validate_hierarchy(entry, &by_path)?;
        if entry.entry_type == WorkspaceEntryType::Symlink {
            validate_target_chain(entry, &by_path)?;
        }
    }
    Ok(())
}

fn validate_hierarchy(
    entry: &WorkspaceEntry,
    entries: &BTreeMap<&str, &WorkspaceEntry>,
) -> Result<(), ResolvedRunError> {
    for ancestor in Path::new(&entry.path).ancestors().skip(1) {
        let Some(ancestor) = ancestor.to_str().filter(|value| !value.is_empty()) else {
            continue;
        };
        if entries
            .get(ancestor)
            .is_some_and(|entry| entry.entry_type != WorkspaceEntryType::Directory)
        {
            return Err(ResolvedRunError::new(
                "workspace manifest contains an unsafe path hierarchy",
            ));
        }
    }
    Ok(())
}

fn validate_target_chain(
    entry: &WorkspaceEntry,
    entries: &BTreeMap<&str, &WorkspaceEntry>,
) -> Result<(), ResolvedRunError> {
    let mut resolved = entry.path.split('/').collect::<Vec<_>>();
    resolved.pop();
    for component in entry
        .symlink_target
        .as_deref()
        .expect("symlink entry has a validated target")
        .split('/')
    {
        match component {
            "." => continue,
            ".." => {
                resolved.pop();
                continue;
            }
            value => resolved.push(value),
        }
        let prefix = resolved.join("/");
        if entries
            .get(prefix.as_str())
            .is_some_and(|entry| entry.entry_type == WorkspaceEntryType::Symlink)
        {
            return Err(ResolvedRunError::new(
                "workspace symlink target traverses another symlink",
            ));
        }
    }
    Ok(())
}
