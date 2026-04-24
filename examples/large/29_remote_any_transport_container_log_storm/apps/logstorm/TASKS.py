# Example: large/29_remote_any_transport_container_log_storm
# File: apps/logstorm/TASKS.py
# Scenario: transport-agnostic remote container log storm

REMOTE = Execution.Remote(
  required_tags=[],
  required_capabilities=["linux"],
  runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(
  tasks=[
    task(
      "container_log_storm",
      deps=["//:prepare_local_input"],
      outputs=[path("//out/container-log-storm-summary.txt")],
      steps=[
        cmd(
          "sh",
          "-c",
          """mkdir -p out
burst=1
stdout_index=1
stderr_index=1
while [ "$burst" -le 3 ]; do
  count=1
  while [ "$count" -le 80 ]; do
    printf 'log-storm-stdout-%03d\n' "$stdout_index"
    stdout_index=$((stdout_index + 1))
    count=$((count + 1))
  done
  count=1
  while [ "$count" -le 20 ]; do
    printf 'log-storm-stderr-%03d\n' "$stderr_index" >&2
    stderr_index=$((stderr_index + 1))
    count=$((count + 1))
  done
  sleep 1
  burst=$((burst + 1))
done
printf 'runtime=%s\nengine=%s\nimage=%s\nstdout_lines=%s\nstderr_lines=%s\nbursts=%s\n' \
  "$TAK_REMOTE_RUNTIME" \
  "$TAK_REMOTE_ENGINE" \
  "$TAK_REMOTE_CONTAINER_IMAGE" \
  "$((stdout_index - 1))" \
  "$((stderr_index - 1))" \
  "3" \
  > out/container-log-storm-summary.txt""",
        )
      ],
      execution=REMOTE,
    ),
    task(
      "observe_container_log_storm",
      deps=[":container_log_storm"],
      steps=[
        cmd(
          "sh",
          "-c",
          "grep -q '^runtime=containerized$' out/container-log-storm-summary.txt && grep -q '^engine=docker$' out/container-log-storm-summary.txt && grep -q '^image=alpine:3.20$' out/container-log-storm-summary.txt && grep -q '^stdout_lines=240$' out/container-log-storm-summary.txt && grep -q '^stderr_lines=60$' out/container-log-storm-summary.txt && grep -q '^bursts=3$' out/container-log-storm-summary.txt && printf 'container-log-storm-verified\n' > out/container-log-storm-verified.txt && cat out/local-input.txt out/container-log-storm-summary.txt out/container-log-storm-verified.txt > out/container-log-storm-report.txt",
        )
      ],
    ),
  ]
)
SPEC
