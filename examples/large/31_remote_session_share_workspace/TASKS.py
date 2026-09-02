# Example: large/31_remote_session_share_workspace
# File: TASKS.py
# Scenario: remote workspace reuse across fresh task invocations

REMOTE = Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=Transport.DirectHttps(),
    container=Container.Image(
        "alpine:3.20",
        resources=Container.Resources(cpu_cores=1.0, memory_mb=512),
    ),
)

WORKSPACE_SESSION = session(
    "workspace-state",
    execution=REMOTE,
    reuse=SessionReuse.SharedWorkspace(max_parallel_tasks=2),
    affinity=Affinity.RequireSameNode("workspace-state"),
)

SPEC = module_spec(
    spec_version=2,
    project_id="example_large_31",
    tasks=[
        task(
            "prepare_workspace",
            outputs=[path("out/prepare-workspace.txt")],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p .session out && "
                    "printf 'prepared\\n' > .session/state.txt && "
                    "printf 'workspace-prepared\\n' > out/prepare-workspace.txt",
                )
            ],
            use_session=WORKSPACE_SESSION,
        ),
        task(
            "verify_workspace",
            deps=[":prepare_workspace"],
            outputs=[path("out/workspace-session.txt")],
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "test -f .session/state.txt && "
                    "mkdir -p out && "
                    "printf 'workspace-state-reused\\n' > out/workspace-session.txt",
                )
            ],
            use_session=WORKSPACE_SESSION,
        ),
    ],
)
SPEC
