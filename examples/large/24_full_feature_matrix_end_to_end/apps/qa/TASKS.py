# Example: large/24_full_feature_matrix_end_to_end
# File: apps/qa/TASKS.py
# Scenario: full feature matrix end to end

SPEC = module_spec(
  tasks=[
    task(
      "validate",
      deps=["//libs/common:lint"],
      needs=[
        need("cpu", 2, scope=MACHINE),
        need("ram_gib", 2, scope=MACHINE),
        need("ui_lock", 1, scope=MACHINE),
        need("start_rl", 1, scope=MACHINE, hold=AT_START),
        need("simulator", 1, scope=MACHINE),
        need("project_gate", 1, scope=PROJECT),
        need("user_gate", 1, scope=USER),
        need("worktree_gate", 1, scope=WORKTREE),
      ],
      queue=queue_use("qa_priority", scope=MACHINE, slots=1, priority=10),
      steps=[cmd("sh", "-c", "echo qa-validate >> out/full_matrix.log")]
    ),
    task(
      "release",
      deps=[":validate"],
      queue=queue_use("qa_fifo", scope=MACHINE, slots=1, priority=0),
      steps=[script("scripts/matrix_release.sh", interpreter="sh")]
    )
  ]
)
SPEC
