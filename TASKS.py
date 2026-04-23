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


LINT_STEPS = [
    cmd("cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
]

TEST_STEPS = [
    cmd("cargo", "build", "-p", "tak", "--bin", "tak"),
    cmd("cargo", "build", "-p", "takd", "--bin", "takd"),
    cmd("cargo", "test", "--workspace", "--lib", "--no-run"),
    cmd("cargo", "test", "-p", "tak-core", "--lib"),
    cmd("cargo", "test", "-p", "tak-loader", "--lib"),
    cmd("cargo", "test", "-p", "tak-proto", "--lib"),
    cmd("cargo", "test", "-p", "tak-runner", "--lib"),
    cmd("cargo", "test", "-p", "tak-exec", "--lib"),
    cmd("cargo", "test", "-p", "takd", "--lib"),
    cmd("cargo", "test", "-p", "tak", "--lib"),
]

DOCS_CHECK_STEPS = [
    cmd("cargo", "test", "--workspace", "--doc"),
]

DOCS_WIKI_STEPS = [
    script("scripts/generate_docs_wiki.sh", interpreter="bash"),
]

COVERAGE_STEPS = [
    script("scripts/run_coverage.sh", interpreter="bash"),
]

CI_COVERAGE_STEPS = [
    script("scripts/run_ci_coverage.sh", interpreter="bash"),
]


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
    tasks=[
        task("fmt-check", steps=FMT_CHECK_STEPS),
        task("line-limits-check", steps=LINE_LIMITS_CHECK_STEPS),
        task("src-test-separation-check", steps=SRC_TEST_SEPARATION_CHECK_STEPS),
        task("workflow-contract-check", steps=WORKFLOW_CONTRACT_CHECK_STEPS),
        task("lint", steps=LINT_STEPS),
        task("test", steps=TEST_STEPS),
        task("docs-check", steps=DOCS_CHECK_STEPS),
        task(
            "docs-wiki",
            doc="Build a MkDocs Material wiki from source-derived Tak docs.",
            outputs=[path(".tmp/docs-wiki")],
            steps=DOCS_WIKI_STEPS,
        ),
        task("check-rust", deps=[":lint", ":test", ":docs-check"]),
        task("coverage", steps=COVERAGE_STEPS),
        task(
            "ci",
            context=CurrentState(ignored=[gitignore()]),
            outputs=[],
            deps=[
                ":fmt-check",
                ":line-limits-check",
                ":src-test-separation-check",
                ":workflow-contract-check",
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
            context=CurrentState(ignored=[gitignore()]),
            outputs=[],
            deps=[
                ":fmt-check",
                ":line-limits-check",
                ":src-test-separation-check",
                ":workflow-contract-check",
                ":check-rust",
            ],
        ),
    ],
)
SPEC
