use std::collections::BTreeMap;

use takd::RemoteRuntimeConfig;

use super::RuntimeConfigBuilder;

const ISOLATED_DOCKER_HOST: &str = "unix:///nonexistent/takd-tests-isolated-docker.sock";

pub(super) fn build(options: RuntimeConfigBuilder) -> RemoteRuntimeConfig {
    let mut values = BTreeMap::new();
    values.insert(
        "DOCKER_HOST",
        options
            .docker_host
            .unwrap_or_else(|| ISOLATED_DOCKER_HOST.to_string()),
    );
    values.insert("TAKD_MEMORY_PRESSURE_ENABLED", "false".to_string());
    values.insert("TAKD_ADMISSION_OVERSUBSCRIBE_X", "1".to_string());
    if let Some(cpu_cores) = options.default_container_cpu_cores {
        values.insert("TAKD_DEFAULT_CONTAINER_CPU_CORES", cpu_cores.to_string());
    }
    if let Some(memory_mb) = options.default_container_memory_mb {
        values.insert("TAKD_DEFAULT_CONTAINER_MEMORY_MB", memory_mb.to_string());
    }

    set_path(
        &mut values,
        "TAKD_REMOTE_EXEC_ROOT",
        options.explicit_remote_exec_root,
    );
    if options.skip_exec_root_probe {
        values.insert("TAK_TEST_HOST_PLATFORM", "other".to_string());
    }
    set_duration(
        &mut values,
        "TAKD_REMOTE_CLEANUP_TTL_MS",
        options.remote_cleanup_ttl,
    );
    set_duration(
        &mut values,
        "TAKD_REMOTE_CLEANUP_INTERVAL_MS",
        options.remote_cleanup_interval,
    );
    set_duration(
        &mut values,
        "TAKD_REMOTE_CLIENT_STALE_TTL_MS",
        options.remote_client_stale_ttl,
    );
    set_duration(
        &mut values,
        "TAKD_REMOTE_CLIENT_WATCHDOG_INTERVAL_MS",
        options.remote_client_watchdog_interval,
    );
    let temp_dir = options.temp_dir.unwrap_or_else(std::env::temp_dir);
    RemoteRuntimeConfig::from_environment(|key| values.get(key).cloned(), temp_dir, true)
}

fn set_path(
    values: &mut BTreeMap<&str, String>,
    key: &'static str,
    value: Option<std::path::PathBuf>,
) {
    if let Some(value) = value {
        values.insert(key, value.display().to_string());
    }
}

fn set_duration(
    values: &mut BTreeMap<&str, String>,
    key: &'static str,
    value: Option<std::time::Duration>,
) {
    if let Some(value) = value {
        values.insert(key, value.as_millis().to_string());
    }
}
