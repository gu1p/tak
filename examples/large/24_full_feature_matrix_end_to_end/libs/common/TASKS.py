# Example: large/24_full_feature_matrix_end_to_end
# File: libs/common/TASKS.py
# Scenario: full feature matrix end to end

SPEC = module_spec(
  tasks=[
    task("lint", deps=["//:seed_flaky"], steps=[cmd("sh", "-c", "mkdir -p out && echo common-lint >> out/full_matrix.log")])
  ]
)
SPEC
