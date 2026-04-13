FMT_CHECK_STEPS = [
    cmd("cargo", "fmt", "--all", "--", "--check"),
]

LINE_LIMITS_CHECK_STEPS = [
    cmd("bash", "scripts/check_rust_file_limits.sh"),
]

SRC_TEST_SEPARATION_CHECK_STEPS = [
    cmd("bash", "scripts/check_no_tests_in_src.sh"),
]

WORKFLOW_CONTRACT_CHECK_STEPS = [
    cmd("bash", "scripts/check_workflow_binary_matrix.sh"),
]

GENERATED_ARTIFACT_IGNORE_CHECK_STEPS = [
    cmd("bash", "scripts/check_generated_artifacts_ignore.sh"),
]

LINT_STEPS = [
    cmd("cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
]

TEST_STEPS = [
    cmd("cargo", "test", "--workspace"),
]

DOCS_CHECK_STEPS = [
    cmd("cargo", "test", "--workspace", "--doc"),
    cmd("cargo", "test", "-p", "tak", "--test", "doctest_contract"),
]

SPEC = module_spec(
    project_id="tak",
    defaults={
        "container_runtime": DockerfileRuntime(
            dockerfile=path("docker/tak-tests/Dockerfile"),
            build_context=path("docker/tak-tests"),
        ),
    },
    tasks=[
        task("fmt-check", steps=FMT_CHECK_STEPS),
        task("line-limits-check", steps=LINE_LIMITS_CHECK_STEPS),
        task("src-test-separation-check", steps=SRC_TEST_SEPARATION_CHECK_STEPS),
        task("workflow-contract-check", steps=WORKFLOW_CONTRACT_CHECK_STEPS),
        task("generated-artifact-ignore-check", steps=GENERATED_ARTIFACT_IGNORE_CHECK_STEPS),
        task("lint", steps=LINT_STEPS),
        task("test", steps=TEST_STEPS),
        task("docs-check", steps=DOCS_CHECK_STEPS),
        task(
            "check",
            context=CurrentState(ignored=[gitignore()]),
            outputs=[],
            steps=FMT_CHECK_STEPS
            + LINE_LIMITS_CHECK_STEPS
            + SRC_TEST_SEPARATION_CHECK_STEPS
            + WORKFLOW_CONTRACT_CHECK_STEPS
            + GENERATED_ARTIFACT_IGNORE_CHECK_STEPS
            + LINT_STEPS
            + TEST_STEPS
            + DOCS_CHECK_STEPS,
        ),
    ],
)
SPEC
