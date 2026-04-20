use std::fs;

use tak_loader::{LoadOptions, load_workspace};

#[test]
fn workspace_load_accepts_named_stub_shapes_for_tasks_surface() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("docker")).expect("docker dir");
    fs::create_dir_all(temp.path().join("scripts")).expect("scripts dir");
    fs::create_dir_all(temp.path().join("src")).expect("src dir");
    fs::write(temp.path().join("docker/Dockerfile"), "FROM alpine:3.20\n")
        .expect("write dockerfile");
    fs::write(temp.path().join("scripts/check.sh"), "echo ok\n").expect("write script");
    fs::write(temp.path().join("src/lib.txt"), "typed stub fixture\n").expect("write src file");

    fs::write(
        temp.path().join("TASKS.py"),
        r#"build: TaskSpec = task(
  "build",
  steps=[cmd("sh", "-c", "echo build")],
)
default_retry: RetrySpec = retry(
  attempts=2,
  on_exit=[44],
  backoff=exp_jitter(min_s=1, max_s=2, jitter="full"),
)
default_runtime: DockerfileRuntimeSpec = DockerfileRuntime(
  dockerfile=path("docker/Dockerfile"),
)
remote: RemoteSpec = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=DirectHttps(),
  runtime=ContainerRuntime(image="alpine:3.20"),
)
context: CurrentStateSpec = CurrentState(
  roots=[path("src")],
  ignored=[gitignore()],
  include=[path("scripts/check.sh")],
)
check: TaskSpec = task(
  "check",
  deps=[build],
  steps=[
    cmd("sh", "-c", "echo check"),
    script("scripts/check.sh", interpreter="bash"),
  ],
  needs=[
    need("cpu", 1, scope=MACHINE),
    need("ui_lock", 1, scope=MACHINE, hold=AT_START),
  ],
  queue=queue_use("qa", scope=MACHINE, slots=1, priority=1),
  retry=retry(attempts=2, on_exit=[42], backoff=fixed(0.2)),
  timeout_s=120,
  context=context,
  outputs=[path("out"), glob("dist/*.txt")],
  execution=RemoteOnly(remote),
  tags=["typed-surface"],
)
spec: ModuleSpec = module_spec(
  project_id="typed_stub_contract",
  tasks=[build, check],
  limiters=[
    resource("cpu", 8, unit="slots", scope=MACHINE),
    lock("ui_lock", scope=MACHINE),
  ],
  queues=[queue_def("qa", slots=1, discipline=FIFO, scope=MACHINE)],
  defaults={
    "retry": default_retry,
    "tags": ["default-tag"],
    "container_runtime": default_runtime,
  },
)
SPEC = spec
SPEC
"#,
    )
    .expect("write tasks");

    let spec = load_workspace(temp.path(), &LoadOptions::default()).expect("load workspace");
    let labels = spec.tasks.keys().map(canonical_label).collect::<Vec<_>>();

    assert!(
        labels.iter().any(|label| label == "//:build"),
        "missing build label: {labels:?}"
    );
    assert!(
        labels.iter().any(|label| label == "//:check"),
        "missing check label: {labels:?}"
    );
}

fn canonical_label(label: &tak_core::model::TaskLabel) -> String {
    match label.package.as_str() {
        "//" => format!("//:{}", label.name),
        _ => format!("{}:{}", label.package, label.name),
    }
}
