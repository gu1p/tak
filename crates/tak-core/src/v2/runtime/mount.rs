use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

impl ContainerMount {
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        read_only: bool,
    ) -> Result<Self, String> {
        let source = source.into();
        validate_workspace_source(&source)?;
        let normalized =
            crate::model::normalize_runtime_mounts(&[crate::model::ContainerMountDef {
                source,
                target: target.into(),
                read_only,
            }])
            .map_err(|error| error.to_string())?
            .pop()
            .expect("one mount definition produces one mount");
        Ok(Self {
            source: normalized.source,
            target: normalized.target,
            read_only: normalized.read_only,
        })
    }
}

pub(super) fn validate_mounts(mounts: &[ContainerMount]) -> Result<(), String> {
    let mut canonical = mounts
        .iter()
        .map(|mount| {
            ContainerMount::new(mount.source.clone(), mount.target.clone(), mount.read_only)
        })
        .collect::<Result<Vec<_>, _>>()?;
    canonical.sort();
    canonical.dedup();
    if canonical != mounts {
        return Err("container runtime mounts are not canonical".into());
    }
    Ok(())
}

fn validate_workspace_source(source: &str) -> Result<(), String> {
    let normalized = source.trim().replace('\\', "/");
    let has_drive_prefix = normalized.as_bytes().get(1) == Some(&b':')
        && normalized
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic);
    if normalized.is_empty()
        || normalized.starts_with('/')
        || has_drive_prefix
        || normalized.split('/').any(|segment| segment == "..")
    {
        return Err(format!(
            "container mount source `{source}` must be workspace-relative; daemon-owned execution cannot mount daemon or worker host paths"
        ));
    }
    Ok(())
}
