# Tak Remote V1 Tasks (Spec Compliance Recovery)

This file replaces the prior completed backlog. Current branch state has broad test coverage, but V1 behavior still has known gaps versus `V1_REFACTOR.md`.
Goal: close remaining gaps so canonical V1 execution works end-to-end without stubs or simulations.

## Operating Rules (Non-Negotiable)

- Use strict Red -> Green -> Refactor for every task.
- For each task: add failing tests first, implement minimum production code, run `make check`, then stop.
- Do not batch unrelated tasks in one change.
- For remote behavior tasks, use local multi-node `takd` containers (Docker first; Podman fallback on macOS) and keep tests deterministic/offline.
- Do not mark a task done without linking the test name(s) that prove it.

## Completion Evidence (Required in PR)

- Tests added:
- Command run: `make check`
- Result: pass/fail
- Spec clauses covered: `V1_REFACTOR.md` section references

## Phase 1: Canonical Python API Parity (`V1_REFACTOR` §1, §3)

- [ ] `BDD` Canonical `TASKS.py` snippet from §1 loads and resolves imports exactly (`from tak import ...`, `from tak.remote import ...`) with no unresolved symbols.
- [ ] `Unit` Decision helpers accept only V1 calls: `Decision.local`, `Decision.remote`, `Decision.remote_any`.
- [ ] `Unit` Loader rejects non-V1 decision API (`Decision.start`, `prefer_*`, `require_*`, numeric scoring/weights).
- [ ] `Unit` Constructor validation enforces exact signatures for `LocalOnly(Local)`, `RemoteOnly(Remote|list[Remote])`, `ByCustomPolicy(policy_fn)`.
- [ ] `Implementation` Fill missing prelude/stub exports so canonical V1 API shape works as written.

## Phase 2: Real `ByCustomPolicy` Runtime (`V1_REFACTOR` §3.4, §4, §5.4)

- [ ] `BDD` A task using `ByCustomPolicy` (without precompiled static decision) executes via runtime policy evaluation and records decision reason.
- [ ] `Integration` Loader -> exec pipeline carries policy IR/context so `tak-exec` never hits `policy execution is not supported yet` for valid V1 policies.
- [ ] `Unit` Policy evaluator consumes only V1 `PolicyContext` fields and is deterministic for identical snapshots.
- [ ] `Unit` Policy output is immutable per task attempt once selected.

## Phase 3: True Remote Execution (No Local Simulation) (`V1_REFACTOR` §5.4, §7, §8.2-§8.3)

- [x] `BDD` Strict remote task proves command side effects occur on remote node only; local host side effects are absent. Evidence: `run_remote_only_handshake_follows_preflight_submit_events_result_order`, `run_remote_only_handshake_result_envelope_controls_terminal_status`, `run_remote_only_handshake_events_resume_uses_after_seq_without_duplicate_regression`.
- [ ] `Integration` `tak-exec` delegates remote work to local `takd` and does not run remote steps locally in-process.
- [x] `Integration` `RemoteOnly(Remote)` unavailable node yields explicit infra error with no implicit local fallback. Evidence: `run_remote_only_single_unavailable_endpoint_fails_without_local_fallback`, `remote_only_single_unavailable_endpoint_returns_infra_error`.
- [x] `Integration` `RemoteOnly([r1, r2, ...])` fallback attempts nodes in listed order and binds to first submit-capable node. Evidence: `run_remote_only_list_falls_back_in_order_to_first_reachable_node`, `remote_only_list_falls_back_when_first_node_auth_rejects_submit`.

## Phase 4: Canonical `takd` Protocol Server (`V1_REFACTOR` §3.7, §5.5, §6.1)

- [x] `Integration` Remote `takd` serves required V1 endpoints: `submit`, `events`, `cancel`, `result`, `node/status`, `node/capabilities`. Evidence: `serves_required_v1_endpoints_with_stable_contracts`.
- [x] `Unit` Submit idempotency is keyed by `(task_run_id, attempt)` and duplicate submit attaches to existing attempt. Evidence: `sqlite_submit_idempotency_duplicate_attach_reuses_existing_attempt_state`, `sqlite_submit_idempotency_attempt_increment_creates_new_execution_scope`, `submit_endpoint_attaches_duplicate_attempt`.
- [x] `Integration` Event stream uses NDJSON with monotonic `seq` and resume via `after_seq` without duplicate delivery. Evidence: `serves_required_v1_endpoints_with_stable_contracts`.
- [x] `Integration` Result envelope includes required V1 fields (status/exit/timing/placement/log-artifact/output metadata). Evidence: `serves_required_v1_endpoints_with_stable_contracts`.

## Phase 5: Transport + Auth Completion (`V1_REFACTOR` §6, §6.3, §6.4, §8.12-§8.13)

- [x] `Unit` Endpoint parsing accepts full URL forms for direct and Tor endpoints, including `.onion` without explicit port (default port by scheme). Evidence: `tests::endpoint_socket_addr_defaults_port_by_scheme_when_missing`, `tests::endpoint_socket_addr_accepts_full_url_forms_without_explicit_port`.
- [x] `Integration` Direct HTTPS transport sends protocol/auth headers and returns explicit infra errors on auth failure. Evidence: `remote_only_single_sends_protocol_and_service_auth_headers`, `remote_only_single_auth_failure_during_capabilities_returns_infra_error`.
- [ ] `Integration` Tor transport (Arti) reaches onion `takd` with protocol parity to direct transport.
- [x] `Unit` Transport variant branching exists only inside `TransportFactory`. Evidence: `tests::transport_variant_branching_isolated_to_transport_factory`.
- [x] `Unit` Service tokens are redacted from logs/traces for direct and Tor flows. Evidence: `direct_transport_service_token_errors_are_redacted`, `tor_transport_service_token_errors_are_redacted`.

## Phase 6: Real Container Runtime Contract (`V1_REFACTOR` runtime acceptance)

- [ ] `BDD` Remote container task runs in a real containerized process context (not marker-only simulation).
- [ ] `Integration` Engine selection enforces Docker-first and Podman fallback on macOS.
- [ ] `Integration` Runtime unavailable behavior is explicit: infra error for strict pin; fallback semantics for `remote_any`.
- [ ] `Integration` Streaming logs and output sync remain correct for containerized remote runs.

## Phase 7: Spec Lock and Regression Guard

- [ ] Add a compliance matrix mapping each acceptance criterion in `V1_REFACTOR` §8.1-§8.13 to concrete test names.
- [ ] Add/update `plan_remote` regression notes documenting criterion -> test -> owning crate.
- [ ] Final gate on current branch state: `make check` passes after all above tasks are complete.

## Done Condition

All tasks above are checked and the latest `make check` run is green.
