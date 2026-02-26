# Example: medium/17_scope_user_project_worktree_mix
# File: TASKS.py
# Scenario: scope user project worktree mix

SPEC = module_spec(
  tasks=[
    task(
      "scoped_task",
      needs=[
        need("user_gate", 1, scope=USER),
        need("project_gate", 1, scope=PROJECT),
        need("worktree_gate", 1, scope=WORKTREE),
      ],
      steps=[cmd("sh", "-c", "mkdir -p out && echo scoped > out/scopes.txt")]
    )
  ]
)
SPEC
