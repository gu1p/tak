# Example: large/31_remote_session_share_workspace
# File: TASKS.py
# Scenario: remote workspace reuse across fresh task invocations

REMOTE = Execution.Remote(
    pool="build",
    required_tags=["builder"],
    required_capabilities=["linux"],
    transport=Transport.DirectHttps(),
    runtime=Runtime.Image("alpine:3.20"),
)

WORKSPACE_SESSION = session(
    "workspace-state",
    execution=REMOTE,
    reuse=SessionReuse.Workspace(),
)

SPEC = module_spec(
    project_id="example_large_31",
    sessions=[WORKSPACE_SESSION],
    tasks=[
        task(
            "prepare_workspace",
            steps=[
                cmd(
                    "sh",
                    "-c",
                    "mkdir -p .session && printf 'prepared\\n' > .session/state.txt",
                )
            ],
            execution=Execution.Session("workspace-state"),
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
            execution=Execution.Session("workspace-state"),
        ),
    ],
)
SPEC
