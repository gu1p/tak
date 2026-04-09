# Tak Client Reset Plan

## Summary

Goal: fix the broken project-local workflow, remove dead `tak.toml` concepts, make CLI errors and output usable, and replace fake E2E with real binary-driven coverage that runs in `make check`.

Core product rules:

- `TASKS.py` is the only project definition file.
- Default workspace is the current directory only.
- No implicit parent traversal.
- No implicit recursive child discovery.
- Cross-directory composition is explicit via `module_spec(includes=[...])`.
- `tak run .` is invalid and must guide the user to valid commands.
- CLI is the primary UX surface.
- Real example runs are part of `make check`.

## Part 1. Local-Only Workspace Loading

- Remove `.git`-root discovery behavior.
- Load only `./TASKS.py` from the current working directory.
- Fail fast with guided errors when `TASKS.py` is missing.
- Stop reading or depending on `tak.toml`.
- Delete example `tak.toml` files and stale references.

Acceptance:

- `tak list` inside `examples/small/01_hello_single_task` loads only that project.
- Duplicate limiter collisions from unrelated projects disappear.
- Running from a directory without `TASKS.py` explains what Tak expected.

## Part 2. Explicit Composition With `includes`

- Extend `module_spec` with `includes=[...]`.
- Support includes that point to directories or explicit `TASKS.py` files.
- Resolve includes relative to the current module file.
- Deduplicate repeated includes.
- Detect and report include cycles.

Acceptance:

- Multi-package examples work through explicit includes.
- Include cycles fail with a specific error that names the path chain.

## Part 3. Actionable CLI Errors

- Reject path-like label input such as `.`, `./...`, and absolute paths.
- Make `tak run .` fail with guidance instead of falling through to loader noise.
- Upgrade duplicate-definition errors to name the conflicting symbol and both source files.
- Normalize missing-workspace and task-not-found messaging.

Acceptance:

- `tak run .` explains valid label syntax and points users to `tak list`.
- Duplicate limiter/task/queue errors name both source files.

## Part 4. CLI Presentation Cleanup

- Keep labels canonical in command output.
- Make human-facing output easier to scan.
- Preserve deterministic command contracts for tests and scripts.
- Keep the current-directory workflow obvious in docs and examples.

Acceptance:

- `list`, `tree`, `explain`, `graph`, and `run` all operate against the local project root.
- Output examples in docs match the shipped CLI behavior.

## Part 5. Real Example Matrix

- Add a catalog-driven integration suite for `examples/catalog.toml`.
- Run the real `tak` binary from each example directory for `list`, `explain`, and `graph`.
- Run `tak run <target>` against isolated staged copies for artifact-producing examples.
- Use live fixtures for local daemon, direct remote, and Tor remote examples.

Acceptance:

- The example matrix catches local workflow regressions.
- Remote examples are exercised through real transports instead of fake shape-only tests.

## Part 6. Wire Into `make check`

- Add the real example matrix to the default quality gate.
- Update docs and READMEs to describe current-directory loading and explicit includes.
- Remove stale references to `tak.toml` and recursive discovery.

Acceptance:

- `make check` validates the new loader contract and the real example matrix.
- Docs match shipped behavior.
