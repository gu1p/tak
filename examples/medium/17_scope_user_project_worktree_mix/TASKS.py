# Example: medium/17_scope_user_project_worktree_mix
# File: TASKS.py
# Scenario: scope user project worktree mix

SPEC = module_spec(
    spec_version=2,
  project_id="example_medium_17",
  limiters=[
    lock("user_gate", scope=Scope.User),
    lock("project_gate", scope=Scope.Project),
    lock("worktree_gate", scope=Scope.Worktree),
  ],
  tasks=[
    task(
      "scoped_task",
      needs=[
        need("user_gate", 1, scope=Scope.User),
        need("project_gate", 1, scope=Scope.Project),
        need("worktree_gate", 1, scope=Scope.Worktree),
      ],
      outputs=[path("out/scopes.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo scoped > out/scopes.txt")]
    )
  ]
)
SPEC
