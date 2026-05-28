use std::fs;
use std::path::Path;

use tak_core::model::{
    ContainerRuntimeSourceSpec, RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec,
    RemoteTransportKind, ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel,
};

pub(super) fn write_remote_inventory(config_root: &Path, content: &str) {
    let tak_dir = config_root.join("tak");
    fs::create_dir_all(&tak_dir).expect("create config");
    fs::write(tak_dir.join("remotes.toml"), content).expect("write inventory");
}

pub(super) fn task() -> ResolvedTask {
    ResolvedTask {
        label: TaskLabel {
            package: "//".into(),
            name: "check".into(),
        },
        doc: String::new(),
        deps: Vec::new(),
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: Default::default(),
        outputs: Vec::new(),
        container_runtime: Some(runtime()),
        execution: TaskExecutionSpec::RemoteOnly(remote_spec(RemoteTransportKind::Tor)),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}

pub(super) fn remote_spec(transport_kind: RemoteTransportKind) -> RemoteSpec {
    RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["linux".into()],
        required_capabilities: vec!["docker".into()],
        transport_kind,
        runtime: Some(runtime()),
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    }
}

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "alpine:3.20".into(),
        },
        resource_limits: None,
    }
}

pub(super) struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    pub(super) fn set(key: &'static str, value: &Path) -> Self {
        let original = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.as_ref() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}
