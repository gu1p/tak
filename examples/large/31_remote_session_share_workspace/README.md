# large/31_remote_session_share_workspace

## Why This Matters

Use `SessionReuse.SharedWorkspace(max_parallel_tasks=2)` when tasks intentionally share mutable
workspace state. The matching `Affinity.RequireSameNode("workspace-state")` is a hard constraint;
tasks launch separately on that node while earlier session writes remain visible.

## Runbook

Bootstrap a direct remote agent, then run:

```bash
tak run //:verify_workspace
```

## Expected Signals

- Run summary includes `session=workspace-state`.
- Run details identify shared-workspace reuse and required same-node affinity.
- `verify_workspace` sees `.session/state.txt` created by `prepare_workspace`.

## Artifacts

- `out/prepare-workspace.txt`
- `out/workspace-session.txt`
