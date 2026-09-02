# Example: large/24_full_feature_matrix_end_to_end
# File: libs/common/TASKS.py
# Scenario: full feature matrix end to end

SPEC = module_spec(
    spec_version=2,
  tasks=[
    task(
      "lint",
      deps=["//:seed_flaky"],
      outputs=[path("//out/full-common-lint.txt")],
      steps=[cmd("sh", "-c", "mkdir -p out && echo common-lint > out/full-common-lint.txt", cwd="//")],
    )
  ]
)
SPEC
