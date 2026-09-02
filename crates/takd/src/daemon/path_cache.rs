use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use tak_core::v2::OutputSelector;

pub(in crate::daemon) mod lock;
mod tree;

pub(in crate::daemon) const ACCESS_MARKER: &str = ".last-accessed-ms";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Publication {
    Published,
    GenerationChanged,
}

pub(super) struct PathCache {
    root: PathBuf,
    selectors: Vec<OutputSelector>,
    _lease: Option<lock::CacheLock>,
}

#[derive(Clone, Copy)]
pub(super) struct Snapshot {
    generation: u64,
}

impl PathCache {
    pub(super) fn new(root: PathBuf, selectors: Vec<OutputSelector>) -> Result<Self> {
        if selectors.is_empty() {
            bail!("SessionReuse.Paths requires at least one path");
        }
        tree::validate_selectors(&selectors)?;
        Ok(Self {
            root,
            selectors,
            _lease: None,
        })
    }

    pub(super) fn new_leased(
        root: PathBuf,
        selectors: Vec<OutputSelector>,
        lease_path: &Path,
    ) -> Result<Self> {
        let mut cache = Self::new(root, selectors)?;
        cache._lease = Some(lock::CacheLock::acquire_shared(lease_path)?);
        Ok(cache)
    }

    pub(super) fn restore_into(&self, workspace: &Path) -> Result<Snapshot> {
        self.prepare_root()?;
        let _guard = lock::CacheLock::acquire(&self.root.join("cache.lock"))?;
        let generation = self.current_generation()?;
        if generation > 0 {
            tree::overlay(&self.generation_root(generation), workspace)?;
        }
        self.touch_access()?;
        Ok(Snapshot { generation })
    }

    pub(super) fn publish_from(&self, workspace: &Path, snapshot: Snapshot) -> Result<Publication> {
        self.prepare_root()?;
        let temporary = self.root.join(format!("pending-{}", uuid::Uuid::new_v4()));
        tree::capture(workspace, &temporary, &self.selectors)?;
        let _guard = lock::CacheLock::acquire(&self.root.join("cache.lock"))?;
        if self.current_generation()? != snapshot.generation {
            tree::remove(&temporary)?;
            return Ok(Publication::GenerationChanged);
        }
        let next = snapshot.generation.saturating_add(1);
        fs::rename(&temporary, self.generation_root(next))?;
        self.write_generation(next)?;
        self.touch_access()?;
        Ok(Publication::Published)
    }

    fn prepare_root(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("generations"))?;
        Ok(())
    }

    fn generation_root(&self, generation: u64) -> PathBuf {
        self.root.join("generations").join(generation.to_string())
    }

    fn current_generation(&self) -> Result<u64> {
        let path = self.root.join("current");
        match fs::read_to_string(path) {
            Ok(value) => Ok(value.trim().parse()?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    fn write_generation(&self, generation: u64) -> Result<()> {
        let temporary = self
            .root
            .join(format!("current-{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temporary, format!("{generation}\n"))?;
        fs::File::open(&temporary)?.sync_all()?;
        fs::rename(temporary, self.root.join("current"))?;
        fs::File::open(&self.root)?.sync_all()?;
        Ok(())
    }

    fn touch_access(&self) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        fs::write(self.root.join(ACCESS_MARKER), now.to_string())?;
        Ok(())
    }
}
