CARGO_SHARED_ENV_SCRIPT = (
    'TAK_TEST_TMPDIR="/tmp/tak-tests-$TAK_RUN_ID-$TAK_JOB_ID" && '
    'mkdir -p "$TAK_TEST_TMPDIR" .tmp/cargo-home .tmp/cargo-target-local && '
    'TMPDIR="$TAK_TEST_TMPDIR" '
    'CARGO_HOME="$PWD/.tmp/cargo-home" '
    'CARGO_TARGET_DIR="$PWD/.tmp/cargo-target-local" '
    "CARGO_INCREMENTAL=0 "
    "CARGO_PROFILE_DEV_DEBUG=0 "
    "CARGO_PROFILE_TEST_DEBUG=0 "
    'CARGO_BUILD_JOBS=2 exec "$@"'
)

CARGO_CHECK_LOCK = "cargo-check-workspace"


def cargo_needs():
    return [need(CARGO_CHECK_LOCK, 1, scope=Scope.Machine)]


def cargo_cmd(*argv):
    return cmd("sh", "-c", CARGO_SHARED_ENV_SCRIPT, "tak-cargo", "cargo", *argv)


CHECK_EXAMPLE_CATALOG_FIXTURES = [
    path("//examples/large/27_hybrid_local_remote_test_suite_success/TASKS.py"),
    path("//examples/large/27_hybrid_local_remote_test_suite_success/apps/web/TASKS.py"),
    path("//examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/TASKS.py"),
    path("//examples/large/28_hybrid_local_remote_test_suite_failure_with_logs/apps/web/TASKS.py"),
    path("//examples/large/29_remote_any_transport_container_log_storm/TASKS.py"),
    path("//examples/large/29_remote_any_transport_container_log_storm/apps/logstorm/TASKS.py"),
    path("//examples/large/30_remote_session_share_paths/TASKS.py"),
    path("//examples/large/31_remote_session_share_workspace/TASKS.py"),
]

CHECK_CONTEXT = CurrentState(
    ignored=[gitignore()],
    include=CHECK_EXAMPLE_CATALOG_FIXTURES,
)

CHECK_CONTAINER = Container.Dockerfile(
    path("docker/tak-tests/Dockerfile"),
    build_context=path("docker/tak-tests"),
)

CHECK_POLICY = Execution.Remote(
    required_tags=["builder"],
    required_capabilities=["linux"],
    container=CHECK_CONTAINER,
    selection=RemoteSelection.Balanced(),
)

CHECK_SESSION = session(
    "check-distributed",
    execution=CHECK_POLICY,
    reuse=SessionReuse.Paths(
        [path(".tmp/cargo-home"), path(".tmp/cargo-target-local")]
    ),
    context=CHECK_CONTEXT,
)

CHECK_ISOLATED_SESSION = session(
    "check-isolated",
    execution=CHECK_POLICY,
    reuse=SessionReuse.Workspace(),
    context=CHECK_CONTEXT,
)


def release_build_task(name, target, build_mode):
    release_dir = ".tmp/release-target/" + target + "/" + target + "/release/"
    return task(
        name,
        doc="Build tak and takd release binaries for " + target + ".",
        execution=Execution.Local(),
        outputs=[path(release_dir + "tak"), path(release_dir + "takd")],
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
        execution=Execution.Local(),
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
    spec_version=2,
    project_id="tak",
    defaults=Defaults(container=CHECK_CONTAINER),
    limiters=[lock(CARGO_CHECK_LOCK, scope=Scope.Machine)],
    tasks=[
        task(
            "fmt-check",
            needs=cargo_needs(),
            steps=[cargo_cmd("fmt", "--all", "--", "--check")],
            use_session=CHECK_SESSION,
        ),
        task(
            "line-limits-check",
            steps=[
                cmd(
                    "bash",
                    "scripts/check_rust_file_limits.sh",
                    env={"TAK_LINE_MODE": "all"},
                )
            ],
            use_session=CHECK_ISOLATED_SESSION,
        ),
        task(
            "src-test-separation-check",
            steps=[
                cmd(
                    "bash",
                    "scripts/check_no_tests_in_src.sh",
                    env={"TAK_LINE_MODE": "all"},
                )
            ],
            use_session=CHECK_ISOLATED_SESSION,
        ),
        task(
            "workflow-contract-check",
            steps=[cmd("bash", "scripts/check_workflow_binary_matrix.sh")],
            use_session=CHECK_ISOLATED_SESSION,
        ),
        task(
            "generated-artifact-ignore-check",
            steps=[cmd("bash", "scripts/check_generated_artifacts_ignore.sh")],
            use_session=CHECK_ISOLATED_SESSION,
        ),
        task(
            "lint",
            needs=cargo_needs(),
            steps=[cargo_cmd("clippy", "--workspace", "--all-targets", "--", "-D", "warnings")],
            use_session=CHECK_SESSION,
        ),
        task(
            "test",
            needs=cargo_needs(),
            steps=[
                cargo_cmd(
                    "test",
                    "--workspace",
                    "--lib",
                    "--tests",
                    "--",
                    "--test-threads=1",
                )
            ],
            use_session=CHECK_SESSION,
        ),
        task(
            "docs-check",
            needs=cargo_needs(),
            steps=[
                cargo_cmd("test", "--workspace", "--doc"),
                cargo_cmd("test", "-p", "tak", "--test", "doctest_contract"),
                cargo_cmd(
                    "test",
                    "-p",
                    "tak",
                    "--test",
                    "suite",
                    "docs_dump_no_drift_contract",
                ),
            ],
            use_session=CHECK_SESSION,
        ),
        task(
            "docs-wiki",
            doc="Build a Zensical wiki from source-derived Tak docs and embedded rustdoc internals.",
            needs=cargo_needs(),
            outputs=[path(".tmp/docs-wiki")],
            steps=[
                cargo_cmd("build", "-p", "tak", "--bin", "tak"),
                cmd("python3", "scripts/docs_wiki.py", "build"),
            ],
        ),
        task(
            "docs-wiki-serve",
            doc="Preview the Zensical wiki generated from source-derived Tak docs and rustdoc internals.",
            needs=cargo_needs(),
            steps=[
                cargo_cmd("build", "-p", "tak", "--bin", "tak"),
                cmd("python3", "scripts/docs_wiki.py", "serve"),
            ],
        ),
        task(
            "check-rust",
            deps=[":lint", ":test", ":docs-check"],
            use_session=CHECK_ISOLATED_SESSION,
        ),
        task("coverage", steps=[script("scripts/run_coverage.sh", interpreter="bash")]),
        task(
            "ci",
            context=CHECK_CONTEXT,
            outputs=[path(".tmp/coverage/lcov.info")],
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
            doc="Run every check with independent remote placement.",
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
            use_session=CHECK_ISOLATED_SESSION,
        ),
    ],
)
SPEC
