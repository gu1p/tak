use super::*;

#[derive(Debug)]
pub(crate) struct ContainerStepSpec {
    pub(crate) argv: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) env: BTreeMap<String, String>,
}

pub(super) struct ContainerStepRunContext<'a> {
    pub(super) workspace_root: &'a Path,
    pub(super) task_label: &'a TaskLabel,
    pub(super) task_run_id: &'a str,
    pub(super) attempt: u32,
    pub(super) output_observer: Option<&'a Arc<dyn TaskOutputObserver>>,
    pub(super) container_user: Option<&'a str>,
    pub(super) cancellation: &'a RunCancellation,
    pub(super) container_identity: Option<&'a crate::ContainerExecutionIdentity>,
    /// Wall-clock step timeout, surfaced as a `tak.timeout_s` container label so
    /// the daemon's memory-pressure controller can avoid pausing a container
    /// whose timeout keeps counting while frozen (which would fail the step).
    pub(super) timeout_s: Option<u64>,
}

pub(super) struct ContainerStepExecutor<'a> {
    pub(super) docker: &'a Docker,
    pub(super) engine: ContainerEngine,
    pub(super) podman_wait_socket: Option<&'a str>,
    pub(super) image: &'a str,
    pub(super) resource_limits: Option<&'a ContainerResourceLimitsSpec>,
}
