CARGO_SHARED_ENV_SCRIPT = (
    'mkdir -p /var/tmp/tak-tests .tmp/cargo-home && '
    'TMPDIR="/var/tmp/tak-tests" CARGO_HOME="$PWD/.tmp/cargo-home" exec "$@"'
)


def cargo_cmd(*argv):
    return cmd("sh", "-c", CARGO_SHARED_ENV_SCRIPT, "tak-cargo", "cargo", *argv)


FMT_CHECK_STEPS = [
    cargo_cmd("fmt", "--all", "--", "--check"),
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
    cargo_cmd("clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
]

TEST_STEPS = [
    cargo_cmd("test", "--workspace"),
]

DOCS_CHECK_STEPS = [
    cargo_cmd("test", "--workspace", "--doc"),
    cargo_cmd("test", "-p", "tak", "--test", "doctest_contract"),
]

DOCS_WIKI_STEPS = [
    cargo_cmd("build", "-p", "tak", "--bin", "tak"),
    cmd("python3", "scripts/docs_wiki.py", "build"),
]

DOCS_WIKI_SERVE_STEPS = [
    cargo_cmd("build", "-p", "tak", "--bin", "tak"),
    cmd("python3", "scripts/docs_wiki.py", "serve"),
]

COVERAGE_STEPS = [
    script("scripts/run_coverage.sh", interpreter="bash"),
]

CI_COVERAGE_STEPS = [
    script("scripts/run_ci_coverage.sh", interpreter="bash"),
]

CHECK_CONTEXT = CurrentState(ignored=[gitignore()])

CHECK_RUNTIME = Runtime.Dockerfile(
    path("docker/tak-tests/Dockerfile"),
    build_context=path("docker/tak-tests"),
)

CHECK_SESSION = session(
    "check-workspace",
    execution=Execution.Local(runtime=CHECK_RUNTIME),
    reuse=SessionReuse.Workspace(),
    context=CHECK_CONTEXT,
)


def release_build_task(name, target, build_mode):
    return task(
        name,
        doc="Build tak and takd release binaries for " + target + ".",
        steps=[
            script(
                "scripts/build_release_target.sh",
                target,
                build_mode,
                interpreter="bash",
            )
        ],
    )


def release_package_task(name, build_task_name, target):
    return task(
        name,
        doc="Package tak and takd release binaries for " + target + ".",
        deps=[":" + build_task_name],
        outputs=[path("dist-manual")],
        steps=[
            script(
                "scripts/package_release_target.sh",
                target,
                interpreter="bash",
            )
        ],
    )


BUILD_RELEASE_X86_64_UNKNOWN_LINUX_MUSL = release_build_task(
    "build-release-x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "zigbuild",
)

BUILD_RELEASE_AARCH64_UNKNOWN_LINUX_MUSL = release_build_task(
    "build-release-aarch64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
    "zigbuild",
)

BUILD_RELEASE_X86_64_APPLE_DARWIN = release_build_task(
    "build-release-x86_64-apple-darwin",
    "x86_64-apple-darwin",
    "build",
)

BUILD_RELEASE_AARCH64_APPLE_DARWIN = release_build_task(
    "build-release-aarch64-apple-darwin",
    "aarch64-apple-darwin",
    "build",
)

PACKAGE_RELEASE_X86_64_UNKNOWN_LINUX_MUSL = release_package_task(
    "package-release-x86_64-unknown-linux-musl",
    "build-release-x86_64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
)

PACKAGE_RELEASE_AARCH64_UNKNOWN_LINUX_MUSL = release_package_task(
    "package-release-aarch64-unknown-linux-musl",
    "build-release-aarch64-unknown-linux-musl",
    "aarch64-unknown-linux-musl",
)

PACKAGE_RELEASE_X86_64_APPLE_DARWIN = release_package_task(
    "package-release-x86_64-apple-darwin",
    "build-release-x86_64-apple-darwin",
    "x86_64-apple-darwin",
)

PACKAGE_RELEASE_AARCH64_APPLE_DARWIN = release_package_task(
    "package-release-aarch64-apple-darwin",
    "build-release-aarch64-apple-darwin",
    "aarch64-apple-darwin",
)

SPEC = module_spec(
    project_id="tak",
    defaults={
        "container_runtime": CHECK_RUNTIME,
    },
    sessions=[CHECK_SESSION],
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
            "docs-wiki",
            doc="Build a Zensical wiki from source-derived Tak docs and embedded rustdoc internals.",
            outputs=[path(".tmp/docs-wiki")],
            steps=DOCS_WIKI_STEPS,
        ),
        task(
            "docs-wiki-serve",
            doc="Preview the Zensical wiki generated from source-derived Tak docs and rustdoc internals.",
            steps=DOCS_WIKI_SERVE_STEPS,
        ),
        task("check-rust", deps=[":lint", ":test", ":docs-check"]),
        task("coverage", steps=COVERAGE_STEPS),
        task(
            "ci",
            context=CHECK_CONTEXT,
            outputs=[],
            deps=[
                ":fmt-check",
                ":line-limits-check",
                ":src-test-separation-check",
                ":workflow-contract-check",
                ":generated-artifact-ignore-check",
                ":lint",
                ":docs-check",
            ],
            steps=CI_COVERAGE_STEPS,
        ),
        BUILD_RELEASE_X86_64_UNKNOWN_LINUX_MUSL,
        BUILD_RELEASE_AARCH64_UNKNOWN_LINUX_MUSL,
        BUILD_RELEASE_X86_64_APPLE_DARWIN,
        BUILD_RELEASE_AARCH64_APPLE_DARWIN,
        PACKAGE_RELEASE_X86_64_UNKNOWN_LINUX_MUSL,
        PACKAGE_RELEASE_AARCH64_UNKNOWN_LINUX_MUSL,
        PACKAGE_RELEASE_X86_64_APPLE_DARWIN,
        PACKAGE_RELEASE_AARCH64_APPLE_DARWIN,
        task(
            "check",
            context=CHECK_CONTEXT,
            outputs=[],
            deps=[
                ":fmt-check",
                ":line-limits-check",
                ":src-test-separation-check",
                ":workflow-contract-check",
                ":generated-artifact-ignore-check",
                ":check-rust",
            ],
            execution=Execution.Session("check-workspace", cascade=True),
        ),
    ],
)
SPEC
