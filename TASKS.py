CARGO_SHARED_ENV_SCRIPT = (
    'mkdir -p /var/tmp/tak-tests .tmp/cargo-home .tmp/cargo-target-local && '
    'TMPDIR="/var/tmp/tak-tests" '
    'CARGO_HOME="$PWD/.tmp/cargo-home" '
    'CARGO_TARGET_DIR="$PWD/.tmp/cargo-target-local" exec "$@"'
)


def cargo_cmd(*argv):
    return cmd("sh", "-c", CARGO_SHARED_ENV_SCRIPT, "tak-cargo", "cargo", *argv)


CHECK_CONTEXT = CurrentState(ignored=[gitignore()])

CHECK_CONTAINER = Container.Dockerfile(
    path("docker/tak-tests/Dockerfile"),
    build_context=path("docker/tak-tests"),
)

CHECK_SESSION = session(
    "check-workspace",
    reuse=SessionReuse.Workspace(),
    context=CHECK_CONTEXT,
)

CHECK_WORKSPACE_POLICY = Execution.FirstAvailable(
    placements=[
        Execution.Remote(container=CHECK_CONTAINER, session=CHECK_SESSION),
        Execution.Local(),
    ],
    doc="Run check tasks in a shared remote-first test workspace container.",
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
    defaults=Defaults(container=CHECK_CONTAINER),
    tasks=[
        task("fmt-check", steps=[cargo_cmd("fmt", "--all", "--", "--check")]),
        task("line-limits-check", steps=[cmd("bash", "scripts/check_rust_file_limits.sh")]),
        task(
            "src-test-separation-check",
            steps=[cmd("bash", "scripts/check_no_tests_in_src.sh")],
        ),
        task(
            "workflow-contract-check",
            steps=[cmd("bash", "scripts/check_workflow_binary_matrix.sh")],
        ),
        task(
            "generated-artifact-ignore-check",
            steps=[cmd("bash", "scripts/check_generated_artifacts_ignore.sh")],
        ),
        task(
            "lint",
            steps=[cargo_cmd("clippy", "--workspace", "--all-targets", "--", "-D", "warnings")],
        ),
        task("test", steps=[cargo_cmd("test", "--workspace")]),
        task(
            "docs-check",
            steps=[
                cargo_cmd("test", "--workspace", "--doc"),
                cargo_cmd("test", "-p", "tak", "--test", "doctest_contract"),
            ],
        ),
        task(
            "docs-wiki",
            doc="Build a Zensical wiki from source-derived Tak docs and embedded rustdoc internals.",
            outputs=[path(".tmp/docs-wiki")],
            steps=[
                cargo_cmd("build", "-p", "tak", "--bin", "tak"),
                cmd("python3", "scripts/docs_wiki.py", "build"),
            ],
        ),
        task(
            "docs-wiki-serve",
            doc="Preview the Zensical wiki generated from source-derived Tak docs and rustdoc internals.",
            steps=[
                cargo_cmd("build", "-p", "tak", "--bin", "tak"),
                cmd("python3", "scripts/docs_wiki.py", "serve"),
            ],
        ),
        task("check-rust", deps=[":lint", ":test", ":docs-check"]),
        task("coverage", steps=[script("scripts/run_coverage.sh", interpreter="bash")]),
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
            steps=[script("scripts/run_ci_coverage.sh", interpreter="bash")],
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
            execution=CHECK_WORKSPACE_POLICY,
            cascade_execution=True,
        ),
    ],
)
SPEC
