use std::collections::BTreeMap;
use std::path::Path;

use takd::RemoteRuntimeConfig;

const ISOLATED_DOCKER_HOST: &str = "unix:///nonexistent/tak-exec-tests-isolated-docker.sock";

pub fn isolated(state_root: &Path) -> RemoteRuntimeConfig {
    let values = BTreeMap::from([
        ("DOCKER_HOST", ISOLATED_DOCKER_HOST.to_string()),
        ("TAK_TEST_HOST_PLATFORM", "other".to_string()),
        ("TAKD_MEMORY_PRESSURE_ENABLED", "false".to_string()),
        ("TAKD_ADMISSION_OVERSUBSCRIBE_X", "1".to_string()),
        (
            "TAKD_REMOTE_EXEC_ROOT",
            state_root.join("remote-exec").display().to_string(),
        ),
    ]);
    RemoteRuntimeConfig::from_environment(
        |key| values.get(key).cloned(),
        state_root.to_path_buf(),
        true,
    )
}
