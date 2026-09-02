use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use anyhow::{Context, Result};
use tak_core::v2::WorkspaceDescriptor;
use tak_proto::worker_v2::WorkspaceCacheDisposition;

use super::super::{RemoteNodeContext, worker_cache_gc};
use crate::daemon::path_cache::lock::CacheLock;

mod io;

use io::{transfer_lock, valid_digest, verify, write_and_publish};

pub(in crate::daemon::remote) struct WorkspaceCachePin {
    file: File,
    descriptor: WorkspaceDescriptor,
    _lease: CacheLock,
}

pub(in crate::daemon::remote) fn probe_workspace_cache(
    context: &RemoteNodeContext,
    descriptor: &WorkspaceDescriptor,
) -> Result<bool> {
    WorkspaceCache::new(context)?.probe(descriptor)
}

pub(in crate::daemon::remote) fn store_workspace_cache(
    context: &RemoteNodeContext,
    descriptor: &WorkspaceDescriptor,
    archive: &[u8],
) -> Result<WorkspaceCacheDisposition> {
    WorkspaceCache::new(context)?.store(descriptor, archive)
}

pub(in crate::daemon::remote) fn pin_workspace_cache(
    context: &RemoteNodeContext,
    descriptor: &WorkspaceDescriptor,
) -> Result<Option<WorkspaceCachePin>> {
    WorkspaceCache::new(context)?.pin(descriptor)
}

pub(in crate::daemon::remote) fn cached_workspace_fingerprints(
    context: &RemoteNodeContext,
) -> Result<Vec<String>> {
    if context.state_root().is_none() {
        return Ok(Vec::new());
    }
    WorkspaceCache::new(context)?.fingerprints()
}

impl WorkspaceCachePin {
    pub(super) fn read_verified(&self) -> Result<Vec<u8>> {
        let mut file = self.file.try_clone()?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        verify(&self.descriptor, &bytes)?;
        Ok(bytes)
    }
}

struct WorkspaceCache {
    state_root: PathBuf,
    root: PathBuf,
}

impl WorkspaceCache {
    fn new(context: &RemoteNodeContext) -> Result<Self> {
        let state_root = context
            .state_root()
            .ok_or_else(|| anyhow::anyhow!("worker v2 cache requires a state root"))?;
        Ok(Self {
            state_root: state_root.to_owned(),
            root: state_root.join("worker-v2-workspace-cache"),
        })
    }

    fn probe(&self, descriptor: &WorkspaceDescriptor) -> Result<bool> {
        let _lease = CacheLock::acquire_shared(&self.lock_path(descriptor))?;
        self.probe_locked(descriptor)
    }

    fn probe_locked(&self, descriptor: &WorkspaceDescriptor) -> Result<bool> {
        let path = self.blob_path(descriptor);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if verify(descriptor, &bytes).is_ok() {
            io::touch(&path)?;
            return Ok(true);
        }
        fs::remove_file(path).context("remove corrupt worker workspace cache blob")?;
        Ok(false)
    }

    fn store(
        &self,
        descriptor: &WorkspaceDescriptor,
        archive: &[u8],
    ) -> Result<WorkspaceCacheDisposition> {
        verify(descriptor, archive)?;
        fs::create_dir_all(&self.root)?;
        let lock = transfer_lock(&self.blob_path(descriptor))?;
        let _guard = lock
            .lock()
            .map_err(|_| anyhow::anyhow!("worker workspace transfer lock poisoned"))?;
        let _lease = CacheLock::acquire(&self.lock_path(descriptor))?;
        if self.probe_locked(descriptor)? {
            return Ok(WorkspaceCacheDisposition::Hit);
        }
        self.publish(descriptor, archive)?;
        Ok(WorkspaceCacheDisposition::Stored)
    }

    fn publish(&self, descriptor: &WorkspaceDescriptor, archive: &[u8]) -> Result<()> {
        let temporary = self
            .root
            .join(format!(".transfer-{}", descriptor.manifest.fingerprint));
        let result = write_and_publish(&temporary, &self.blob_path(descriptor), archive);
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn pin(&self, descriptor: &WorkspaceDescriptor) -> Result<Option<WorkspaceCachePin>> {
        let lease = CacheLock::acquire_shared(&self.lock_path(descriptor))?;
        if !self.probe_locked(descriptor)? {
            return Ok(None);
        }
        let file = File::open(self.blob_path(descriptor))?;
        let pin = WorkspaceCachePin {
            file,
            descriptor: descriptor.clone(),
            _lease: lease,
        };
        pin.read_verified()?;
        Ok(Some(pin))
    }

    fn fingerprints(&self) -> Result<Vec<String>> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut values = entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().to_str().map(str::to_owned))
            .filter_map(|name| name.strip_suffix(".tar").map(str::to_owned))
            .filter(|name| valid_digest(name))
            .collect::<Vec<_>>();
        values.sort();
        Ok(values)
    }

    fn blob_path(&self, descriptor: &WorkspaceDescriptor) -> PathBuf {
        self.root
            .join(format!("{}.tar", descriptor.manifest.fingerprint))
    }

    fn lock_path(&self, descriptor: &WorkspaceDescriptor) -> PathBuf {
        worker_cache_gc::workspace_lock_path(&self.state_root, &descriptor.manifest.fingerprint)
    }
}
