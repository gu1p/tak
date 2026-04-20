use crate::daemon::remote::RemoteRuntimeConfig;

pub(super) fn should_skip_probe(config: &RemoteRuntimeConfig) -> bool {
    config.skip_exec_root_probe()
}
