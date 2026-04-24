# large/31_remote_session_share_workspace

## Why This Matters

Use `ShareWorkspace` when every task in a named session should see the same remote workspace filesystem. Each task still launches as a fresh process/container invocation, but files created by earlier session tasks remain available to later ones.

## Runbook

Bootstrap a direct remote agent, then run:

```bash
tak run //:verify_workspace
```

## Expected Signals

- Run summary includes `session=workspace-state`.
- Run summary includes `reuse=share_workspace`.
- `verify_workspace` sees `.session/state.txt` created by `prepare_workspace`.

## Artifacts

- `out/workspace-session.txt`
