use std::collections::BTreeMap;
use std::fs;

use crate::support::container_runtime::simulated_container_runtime_env;
use crate::support::run_tak_expect_success;

#[test]
fn run_command_executes_local_dockerfile_runtime_with_containerized_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("docker")).expect("create docker dir");
    fs::write(
        temp.path().join("docker/Dockerfile"),
        "FROM alpine:3.20\nRUN printf 'built\\n' > /tmp/built.txt\n",
    )
    .expect("write dockerfile");
    fs::write(
        temp.path().join("TASKS.py"),
        r#"
LOCAL = Execution.Local(container=Container.Dockerfile(path("docker/Dockerfile"), resources=Container.Resources(cpu_cores=1.0, memory_mb=512)),
)

SPEC = module_spec(tasks=[
  task(
    "local_container",
    steps=[cmd("sh", "-c", "mkdir -p out && printf '%s\n' \"$TAK_RUNTIME_SOURCE\" > out/runtime-source.txt")],
    execution=LOCAL,
  ),
])
SPEC
"#,
    )
    .expect("write tasks");

    let mut env = BTreeMap::new();
    env.extend(simulated_container_runtime_env(temp.path()));
    let stdout =
        run_tak_expect_success(temp.path(), &["run", "//:local_container"], &env).expect("run");

    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("runtime=containerized"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("runtime_engine=docker"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("out/runtime-source.txt"))
            .expect("read runtime source marker")
            .trim(),
        "dockerfile"
    );
}
