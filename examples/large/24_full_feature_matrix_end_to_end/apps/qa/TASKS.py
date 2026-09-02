# Example: large/24_full_feature_matrix_end_to_end
# File: apps/qa/TASKS.py
# Scenario: full feature matrix end to end

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "validate",
      deps=["//libs/common:lint"],
      needs=[
        need("cpu", 2, scope=Scope.Machine),
        need("ram_gib", 2, scope=Scope.Machine),
        need("ui_lock", 1, scope=Scope.Machine),
        need("start_rl", 1, scope=Scope.Machine, hold=Hold.AtStart),
        need("simulator", 1, scope=Scope.Machine),
        need("project_gate", 1, scope=Scope.Project),
        need("user_gate", 1, scope=Scope.User),
        need("worktree_gate", 1, scope=Scope.Worktree),
      ],
      queue=queue_use("qa_priority", scope=Scope.Machine, slots=1, priority=10),
      outputs=[path("//out/full-qa-validate.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo qa-validate > out/full-qa-validate.txt", cwd="//")]
    ),
    task(
      "release",
      deps=[":validate"],
      queue=queue_use("qa_fifo", scope=Scope.Machine, slots=1, priority=0),
      outputs=[path("//out/full_matrix_release.txt")],
      steps=[script("//scripts/matrix_release.sh", interpreter="sh", cwd="//")]
    )
  ]
)
SPEC
