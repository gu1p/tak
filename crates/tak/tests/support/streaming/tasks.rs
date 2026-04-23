use anyhow::Result;

use super::super::write_tasks;

pub fn write_local_streaming_tasks(root: &std::path::Path) -> Result<()> {
    write_tasks(
        root,
        r#"
stream_local = task(
  "stream_local",
  steps=[
    cmd(
      "sh",
      "-c",
      "printf 'local-stdout\n'; printf 'local-stderr\n' >&2; sleep 2",
    )
  ],
)
SPEC = module_spec(tasks=[stream_local])
SPEC
"#,
    )
}

pub fn write_remote_streaming_tasks(root: &std::path::Path) -> Result<()> {
    write_tasks(
        root,
        r#"
REMOTE = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=DirectHttps(),
  runtime=ContainerRuntime(image="alpine:3.20"),
)

remote_stream = task(
  "remote_stream",
  execution=RemoteOnly(REMOTE),
  steps=[
    cmd(
      "sh",
      "-c",
      "printf 'remote-stdout\n'; printf 'remote-stderr\n' >&2; sleep 2",
    )
  ],
)
SPEC = module_spec(tasks=[remote_stream])
SPEC
"#,
    )
}

pub fn write_remote_waiting_tasks(root: &std::path::Path) -> Result<()> {
    write_tasks(
        root,
        r#"
REMOTE = Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=DirectHttps(),
  runtime=ContainerRuntime(image="alpine:3.20"),
)

remote_wait = task(
  "remote_wait",
  execution=RemoteOnly(REMOTE),
  steps=[
    cmd(
      "sh",
      "-c",
      "sleep 6; printf 'remote-stdout\n'; printf 'remote-stderr\n' >&2; sleep 2",
    )
  ],
)
SPEC = module_spec(tasks=[remote_wait])
SPEC
"#,
    )
}
