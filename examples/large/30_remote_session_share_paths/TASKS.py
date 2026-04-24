# Example: large/30_remote_session_share_paths
# File: TASKS.py
# Scenario: remote Cargo-style cache reuse with explicit shared paths

REMOTE = Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=DirectHttps(),
    runtime=ContainerRuntime(image="alpine:3.20"),
)

CARGO_SESSION = session(
    "cargo-cache",
    execution=RemoteOnly(REMOTE),
    reuse=SharePaths([path("target"), path("out")]),
)

SPEC = module_spec(
    project_id="example_large_30",
    sessions=[CARGO_SESSION],
    tasks=[
        task(
            "cargo_build",
            outputs=[path("out/build-marker.txt")],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p target/debug out && "
                    "printf 'compiled-binary\\n' > target/debug/app && "
                    "printf 'build-complete\\n' > out/build-marker.txt",
                )
            ],
            execution=UseSession("cargo-cache"),
        ),
        task(
            "cargo_test",
            deps=[":cargo_build"],
            outputs=[path("out/test-marker.txt")],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "test -f target/debug/app && "
                    "mkdir -p out && "
                    "printf 'reused-target-cache\\n' > out/test-marker.txt",
                )
            ],
            execution=UseSession("cargo-cache"),
        ),
    ],
)
SPEC
