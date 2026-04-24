# Example: medium/17_scope_user_project_worktree_mix
# File: TASKS.py
# Scenario: scope user project worktree mix

SPEC = module_spec(
  project_id="example_medium_17",
  tasks=[
    task(
      "scoped_task",
      needs=[
        need("user_gate", 1, scope=Scope.User),
        need("project_gate", 1, scope=Scope.Project),
        need("worktree_gate", 1, scope=Scope.Worktree),
      ],
      steps=[cmd("sh", "-c", "mkdir -p out && echo scoped > out/scopes.txt")]
    )
  ]
)
SPEC
