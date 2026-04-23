# Refactor Tasks Audit

## Audit Basis

Scope included: `crates/*/src`, top-level `TASKS.py`, and production-facing build/doc scripts that materially contribute to complexity.

Scope excluded: `crates/*/tests`, `examples/`, `plan_docs/`, vendored assets, generated protobuf output, and `OUT_DIR` artifacts, except where production code is overengineered specifically because of them.

Evidence standard: every task below is tied to exact files, exact symbols, and current call sites in this branch state. Ranking favors deletion yield, duplication removed, abstraction collapsed, and user-visible risk reduction.

## Highest-Value Tasks

| Rank | Task | Why it is highest value | Main files |
| --- | --- | --- | --- |
| 1 | `RF-001` | Three core crates fake modularity with `include!` and hidden shared namespaces. This makes all later complexity harder to untangle. | `crates/tak-core/src/model.rs`, `crates/tak-loader/src/lib.rs`, `crates/tak-exec/src/lib.rs` |
| 2 | `RF-005` | Remote HTTP/Tor request logic is copied across CLI, executor, and daemon probe paths. Drift is already visible. | `crates/tak/src/cli/remote_probe.rs`, `crates/tak/src/cli/remote_status/fetch.rs`, `crates/takd/src/service/tor/probe.rs`, `crates/tak-exec/src/engine/protocol_result_http.rs` |
| 3 | `RF-010` | `tak docs dump` is served by a full mini-subsystem that hand-parses Python and embeds source blobs for one command. | `crates/tak/src/docs.rs`, `crates/tak/src/docs/dsl_parse.rs`, `crates/tak/src/docs/examples_parse.rs`, `crates/tak/build.rs` |
| 4 | `RF-007` | The loader rewrites source text before execution and reserializes runtime objects through JSON. That is complexity piled on top of a self-inflicted boundary problem. | `crates/tak-loader/src/loader/module_eval.rs`, `crates/tak-loader/src/loader/workspace_load_and_policy_eval.rs` |
| 5 | `RF-012` | Remote event streaming stores the same chunk twice, then reconstructs it through a fallback ladder. This is pure dead weight in a hot path. | `crates/takd/src/daemon/remote/worker_submit_execution/output_observer.rs`, `crates/takd/src/daemon/remote/route_events.rs` |

## Detailed Tasks

### RF-001: Collapse the include-wired mega-modules

Priority: High  
Category: Unnecessary abstraction  
Files: `crates/tak-core/src/model.rs`, `crates/tak-loader/src/lib.rs`, `crates/tak-exec/src/lib.rs`  
Symbols: `include!(...)` module assembly in all three files

Why This Exists: the repo uses file splitting without real module boundaries, so many files are visually separate while still sharing one hidden namespace.

Why It Is Unjustified: this buys almost none of the value of real modules. Instead it creates fake separation, implicit imports, include-order coupling, and global visibility inside each crate. The code is harder to reason about precisely because boundaries are only cosmetic.

Evidence:
- `crates/tak-core/src/model.rs:15-28` splices thirteen model files into one namespace.
- `crates/tak-loader/src/lib.rs:36-45` does the same for the entire loader pipeline.
- `crates/tak-exec/src/lib.rs:67-105` pulls nineteen engine fragments into one file-local namespace.
- `crates/tak-exec/src/lib.rs:62-63` needs `#[allow(unused_imports)] use step_runner::resolve_cwd;`, which is a direct symptom of include-driven name leakage instead of explicit module imports.

Simplest Refactor: replace the `include!` seams with real `mod` boundaries and explicit `use` lists. Start with `tak-exec`, because it is the worst offender, then apply the same change to `tak-loader` and `tak-core`.

Expected Deletion / Collapse: delete the include-wired mega-namespace pattern from the three main crates and remove hidden cross-file dependencies.

### RF-002: Delete transport abstraction theater in `tak-exec`

Priority: High  
Category: Premature generalization  
Files: `crates/tak-exec/src/engine/transport.rs`, `crates/tak-exec/src/lib.rs`  
Symbols: `RemoteTransportAdapter`, `DirectHttpsTransportAdapter`, `TorTransportAdapter`, `TransportFactory`, `transport_adapter_for_kind`

Why This Exists: the executor tries to look extensible by wrapping two transport modes in a trait plus a factory.

Why It Is Unjustified: there are only two concrete implementations, one dead method, and one panic branch for `RemoteTransportKind::Any`. This is not a real plugin boundary. It is a glorified `match` statement stretched into five types.

Evidence:
- `crates/tak-exec/src/engine/transport.rs:1-15` defines the trait.
- `crates/tak-exec/src/engine/transport.rs:25-100` provides exactly two implementations.
- `crates/tak-exec/src/engine/transport.rs:112-115` keeps `transport_name` behind `#[allow(dead_code)]`.
- `crates/tak-exec/src/lib.rs:78-85` panics on `RemoteTransportKind::Any`, which means the abstraction does not even model all enum variants safely.
- The only caller of `transport_adapter_for_kind` is `TransportFactory::adapter` in the same file.

Simplest Refactor: replace the trait/factory pair with plain helper functions that switch on `RemoteTransportKind` directly for connect and timeout behavior.

Expected Deletion / Collapse: remove the trait, the two marker structs, the factory, the dead `name` path, and the `Any` panic branch.

### RF-003: Collapse `tak-exec` shuttle structs and duplicated placement state

Priority: High  
Category: Overengineering  
Files: `crates/tak-exec/src/engine/remote_models.rs`, `crates/tak-exec/src/engine/attempt_submit.rs`, `crates/tak-exec/src/engine/attempt_execution.rs`, `crates/tak-exec/src/engine/task_result.rs`  
Symbols: `TaskPlacement`, `RemoteSubmitContext`, `AttemptExecutionContext`, `AttemptExecutionOutcome`

Why This Exists: the executor moves state through multiple tiny structs instead of keeping local variables near the logic that uses them.

Why It Is Unjustified: most of these types are not domain models. They are temporary argument bundles or output bundles with one construction site and one consumer. That is ceremony, not architecture.

Evidence:
- `RemoteSubmitContext` is defined in `remote_models.rs:67-73`, built once in `attempt_submit.rs:92-97`, and consumed only by `fallback_after_auth_submit_failure`.
- `AttemptExecutionContext` is defined in `attempt_execution.rs:1-10` and created once from `run_single_task`.
- `AttemptExecutionOutcome` is defined in `attempt_execution.rs:13-20` and then unpacked immediately in `task_result.rs:1-30`.
- `TaskPlacement` in `remote_models.rs:18-26` stores `remote_node_id`, `strict_remote_target`, and `ordered_remote_targets`, which creates partially-initialized remote state that must be repaired later in `attempt_submit.rs:14-16` and `attempt_submit.rs:102-103`.

Simplest Refactor: split local and remote attempt execution into separate functions, pass concrete parameters directly, and store only the selected remote target instead of both `remote_node_id` and `strict_remote_target`.

Expected Deletion / Collapse: inline `RemoteSubmitContext`, delete `AttemptExecutionOutcome`, shrink `TaskPlacement`, and reduce the number of “fill this in later” states.

### RF-004: Collapse the redundant remote error wrapper stack

Priority: High  
Category: Unnecessary abstraction  
Files: `crates/tak-exec/src/engine/preflight_failure.rs`, `crates/tak-exec/src/engine/remote_http_exchange_error.rs`, `crates/tak-exec/src/engine/remote_submit_failure.rs`, `crates/tak-exec/src/engine/protocol_submit.rs`  
Symbols: `RemoteNodeInfoFailure`, `RemoteNodeInfoFailureKind`, `RemoteHttpExchangeError`, `RemoteHttpExchangeErrorKind`, `RemoteSubmitFailure`, `RemoteSubmitFailureKind`

Why This Exists: the code wants typed failure classification, so it keeps adding wrapper structs around message strings.

Why It Is Unjustified: the wrappers are nearly identical and mostly just carry `kind + message`. Then one wrapper is translated into another, losing structure again. This is busywork around error strings, not meaningful modeling.

Evidence:
- `RemoteHttpExchangeError` in `remote_http_exchange_error.rs:1-43` is just `kind` plus `message`.
- `RemoteSubmitFailure` in `remote_submit_failure.rs:1-19` is just `kind` plus `message`.
- `RemoteNodeInfoFailure` in `preflight_failure.rs:11-74` is just `kind` plus `message`, plus a constructor that translates from `RemoteHttpExchangeError`.
- `protocol_submit.rs:18-78` manually manufactures `RemoteSubmitFailure` values on each branch instead of benefiting from any richer abstraction.

Simplest Refactor: keep one classified remote-request failure enum for the executor path and build user-facing messages at the outer layer instead of wrapping message strings multiple times.

Expected Deletion / Collapse: delete at least two of the three wrapper types and stop converting one string wrapper into another.

### RF-005: Remove cross-crate remote HTTP/Tor copy-paste

Priority: High  
Category: Duplicated logic  
Files: `crates/tak/src/cli/remote_probe.rs`, `crates/tak/src/cli/remote_status/fetch.rs`, `crates/tak/src/cli/remote_probe_support/http.rs`, `crates/takd/src/service/tor/probe.rs`, `crates/tak-exec/src/engine/protocol_result_http.rs`  
Symbols: `probe_node`, `fetch_node_status`, `send_http_get`, `wait_for_tor_hidden_service_startup`, `send_node_info_request`, `remote_protocol_http_request`

Why This Exists: each subsystem reimplemented the same “connect, handshake HTTP/1, send GET, collect body, retry Tor” flow locally.

Why It Is Unjustified: the flow is already obviously shared. The CLI even created `send_http_get`, but the daemon probe and executor ignored it and wrote the same code again.

Evidence:
- `remote_probe.rs:10-42` and `remote_status/fetch.rs:42-74` are near-mirror functions that differ mostly by endpoint path and decode type.
- `remote_probe_support/http.rs:9-45` already centralizes Hyper handshake plus body collection inside the CLI crate.
- `takd/src/service/tor/probe.rs:134-173` duplicates that same handshake/request/collect pattern.
- `tak-exec/src/engine/protocol_result_http.rs:101-191` duplicates it again for remote protocol exchange.
- `AbortOnDrop` exists in `tak/src/cli/remote_probe_support.rs`, `takd/src/service/tor/probe.rs`, and `tak-exec/src/engine/protocol_result_http.rs`.

Simplest Refactor: create one internal exchange helper per crate boundary and force all request paths in that crate through it. Do not introduce a new framework. Do delete duplicate handshake code.

Expected Deletion / Collapse: remove duplicate HTTP/1 exchange code, duplicate `AbortOnDrop` types, and most request-construction boilerplate.

### RF-006: Delete wrapper/helper duplication that only renames shared behavior

Priority: Medium  
Category: Duplicated logic  
Files: `crates/tak-exec/src/remote_endpoint.rs`, `crates/takd/src/service/tor/probe.rs`, `crates/tak/src/cli/remote_probe_support/transport.rs`, `crates/tak-exec/src/engine/transport.rs`, `crates/takd/src/service/tor/startup_policy.rs`, `crates/takd/src/service/tor/health.rs`, `crates/tak/src/cli/remote_status/render.rs`, `crates/tak-exec/src/engine/remote_wait_status.rs`  
Symbols: `endpoint_socket_addr`, `endpoint_host_port`, `test_tor_onion_dial_addr`, `env_duration_ms`, `format_cpu`, `format_memory`, `human_bytes`, `human_remote_wait_bytes`

Why This Exists: the code repeatedly wraps or recopies helpers rather than deciding on one home for shared utilities.

Why It Is Unjustified: these are not domain-specific variations. They are mechanically duplicated helpers with trivial differences in naming or message text.

Evidence:
- `remote_endpoint.rs:28-48` is a forwarding layer over `tak_core::endpoint` plus one env lookup helper.
- `takd/src/service/tor/probe.rs:126-131` repeats the same endpoint forwarding.
- `tak-exec/src/engine/transport.rs:152-158`, `tak/src/cli/remote_probe_support/transport.rs:144-155`, `takd/src/service/tor/startup_policy.rs:24-30`, and `takd/src/service/tor/health.rs:96-104` all reimplement env-to-duration parsing.
- `tak/src/cli/remote_status/render.rs:76-151` and `tak-exec/src/engine/remote_wait_status.rs:66-97` duplicate CPU, memory, and byte formatting logic.

Simplest Refactor: move endpoint parsing and path containment to `tak-core`, keep one env-duration parser per crate at most, and keep one byte-formatting helper per presentation layer.

Expected Deletion / Collapse: delete forwarding wrappers and copy-pasted formatter helpers.

### RF-007: Remove loader compatibility string-rewrite sludge

Priority: High  
Category: Suspicious edge-case handling  
Files: `crates/tak-loader/src/loader/module_eval.rs`, `crates/tak-loader/src/loader/workspace_load_and_policy_eval.rs`  
Symbols: `sanitize_canonical_v1_imports`, `monty_to_json`, `eval_module_spec`, `evaluate_named_policy_decision`

Why This Exists: the loader wants to keep old DSL spellings working, but instead of fixing the DSL boundary it rewrites source text and then serializes Monty values through JSON.

Why It Is Unjustified: source-to-source rewriting with blind `.replace(...)` calls is brittle, silent, and invisible to the user. It only exists because the internal DSL surface is not actually stable enough to execute directly.

Evidence:
- `module_eval.rs:48-98` strips imports and blindly replaces legacy token spellings like `RemoteTransportMode.AnyTransport(` and `Reason.DEFAULT_LOCAL_POLICY`.
- `workspace_load_and_policy_eval.rs:86-119` runs the same rewrite path again for policy evaluation.
- `sanitize_canonical_v1_imports` has exactly two call sites, both internal. This is not a shared language feature. It is loader-local compatibility sludge.
- `monty_to_json` in `module_eval.rs:109-147` converts runtime objects into JSON just so serde can parse them back into Rust types.

Simplest Refactor: keep one canonical DSL surface in the prelude/stubs, reject unsupported legacy forms explicitly, and deserialize from structured runtime values instead of round-tripping through handwritten source rewrites plus JSON.

Expected Deletion / Collapse: delete `sanitize_canonical_v1_imports` entirely and shrink or remove the Monty-to-JSON conversion layer.

### RF-008: Stop revalidating identical defaults and duplicating path containment rules

Priority: High  
Category: Duplicated logic  
Files: `crates/tak-loader/src/loader/module_merge.rs`, `crates/tak-loader/src/loader/remote_validation.rs`, `crates/tak/src/cli/run_override_runtime.rs`  
Symbols: `merge_module`, `validate_runtime`, `is_path_within`, `path_ref_within`

Why This Exists: runtime normalization happens in multiple layers, so each layer rechecks the same invariants.

Why It Is Unjustified: some of the repeated work is provably redundant. `module_merge` revalidates the same `module.defaults.container_runtime` once per task. The CLI then copies the same path-containment logic again.

Evidence:
- `module_merge.rs:84-88` validates `module.defaults.container_runtime.clone()` inside the task loop.
- `remote_validation.rs:91-103` defines `is_path_within`.
- `run_override_runtime.rs:99-112` defines `path_ref_within` with the same rule.
- `validate_runtime` is already used from `execution_resolution.rs` and `module_merge.rs`, so the code already has a shared normalization function but still duplicates its sub-rules elsewhere.

Simplest Refactor: compute validated module defaults once before iterating tasks, and move `PathRef` containment checking into `tak-core` so the loader and CLI stop carrying their own copies.

Expected Deletion / Collapse: remove one per-task revalidation path and one full duplicate path-containment helper.

### RF-009: Collapse CLI execution override churn

Priority: Medium  
Category: Overengineering  
Files: `crates/tak/src/cli/run_overrides.rs`, `crates/tak/src/cli/run_override_runtime.rs`  
Symbols: `apply_run_execution_overrides`, `existing_local_spec`, `existing_remote_spec`, `declared_container_runtime`, `resolve_container_runtime_for_task`

Why This Exists: the CLI tries to preserve every existing execution shape while layering overrides on top.

Why It Is Unjustified: the override path spends too much code unpacking and repacking execution variants. Several helpers exist for one caller each, and the whole workspace is cloned even when only a target closure is touched.

Evidence:
- `apply_run_execution_overrides` in `run_overrides.rs:28-76` clones the full `WorkspaceSpec`.
- `existing_local_spec` and `existing_remote_spec` are each used only inside this one override function.
- `declared_container_runtime` in `run_override_runtime.rs:67-85` exists to restate runtime extraction that the override layer already knows how to read.
- `path_ref_within` duplicates loader logic instead of reusing a core invariant.

Simplest Refactor: replace the helper trio with one `rewrite_execution_for_target` function, stop cloning more state than necessary, and share runtime/path helpers with the loader or `tak-core`.

Expected Deletion / Collapse: remove one-off spec extraction helpers and simplify override rewriting into one narrow path.

### RF-010: Slash the `tak docs dump` overbuilt subsystem

Priority: High  
Category: Overengineering  
Files: `crates/tak/src/docs.rs`, `crates/tak/src/docs/dsl.rs`, `crates/tak/src/docs/dsl_parse.rs`, `crates/tak/src/docs/examples.rs`, `crates/tak/src/docs/examples_parse.rs`, `crates/tak/src/docs/model.rs`, `crates/tak/build.rs`, `crates/tak/src/cli/run_cli.rs`  
Symbols: `render_docs_dump`, `collect_dsl_docs`, `parse_typed_dict_class`, `extract_parenthesized_body`, `extract_keyword_string`, `documented_example_sources`, `render_docs_dump_examples`

Why This Exists: the project wants a source-derived authoring bundle, so it built a full docs extraction pipeline around `tak docs dump`.

Why It Is Unjustified: there is only one product consumer for this subsystem: `run_cli` calls `render_docs_dump` directly. Yet the code embeds example sources at build time, parses Python with handwritten scanners, and renders docs from several parallel metadata paths.

Evidence:
- `run_cli.rs:23-27` shows `render_docs_dump` is called only by the `tak docs dump` command.
- `docs.rs:29-121` stitches together crate docs, CLI docs, DSL docs, project-shape summaries, and embedded example sources in one function.
- `dsl_parse.rs:1-190` hand-parses class definitions, docstrings, and function signatures from Python stubs.
- `examples_parse.rs:3-168` hand-parses parentheses, strings, and keyword assignments from Python source.
- `build.rs:48-107` embeds all documented example `TASKS.py` source into generated Rust so the docs system can re-parse it later.

Simplest Refactor: decide what `tak docs dump` actually needs as structured data, generate or embed only that data, and delete the handwritten Python scanners. If example source excerpts must stay embedded, embed curated metadata instead of entire source files plus parsers.

Expected Deletion / Collapse: delete `examples_parse.rs` outright, shrink `dsl_parse.rs` heavily, and remove the build-time source-embedding detour.

### RF-011: Move the inline docs wiki application out of `TASKS.py`

Priority: High  
Category: Verbose low-signal code  
Files: `TASKS.py`  
Symbols: `DOCS_WIKI_SNIPPET`, `DOCS_WIKI_STEPS`, `DOCS_WIKI_SERVE_STEPS`

Why This Exists: the repo wanted a docs wiki task quickly, so the entire implementation was stuffed into a triple-quoted string and executed via `python3 -c`.

Why It Is Unjustified: `TASKS.py` is supposed to declare work, not hide a multi-hundred-line Python application in a string literal. This destroys readability, diff quality, and local tooling.

Evidence:
- `TASKS.py:34-363` is a full standalone Python program embedded as one string.
- `TASKS.py:365-382` shells out with `python3 -c DOCS_WIKI_SNIPPET`.
- The same file then also carries release-task factories and the root task graph, so the embedded app actively obscures the actual task definitions.

Simplest Refactor: move the docs wiki implementation to `scripts/docs_wiki.py` and replace the `python3 -c` invocation with a normal script step.

Expected Deletion / Collapse: delete the giant inline string blob from `TASKS.py` and restore the file to declarative task wiring.

### RF-012: Stop storing stream chunks twice and rebuilding them through fallbacks

Priority: High  
Category: Dead weight  
Files: `crates/takd/src/daemon/remote/worker_submit_execution/output_observer.rs`, `crates/takd/src/daemon/remote/route_events.rs`  
Symbols: `RemoteWorkerEventObserver::observe_output`, `handle_remote_events_route`

Why This Exists: the daemon stores both a lossy text form and a base64 form for each output chunk, then later tries to recover byte data from whichever field happens to exist.

Why It Is Unjustified: the system already has a byte-safe contract. The duplicate text payload exists only to preserve a second representation that the route then has to distrust.

Evidence:
- `output_observer.rs:57-63` writes both `"chunk"` and `"chunk_base64"` for every stream event.
- `route_events.rs:64-80` reconstructs `chunk_bytes` by trying `chunk_base64`, then `chunk`, then `message`.
- The fallback to `message` is especially bad: it proves the route no longer trusts its own stored schema.

Simplest Refactor: persist one byte-safe field for stdout/stderr chunks and derive text only when the bytes are valid UTF-8 at response/render time.

Expected Deletion / Collapse: delete the duplicate lossy chunk field from stored stream events and remove the fallback ladder in the route.

### RF-013: Delete explicit dead-weight markers and unused hooks

Priority: Medium  
Category: Dead code / dead-weight  
Files: `crates/takd/src/lib.rs`, `crates/tak-exec/src/engine/transport.rs`, `crates/tak-exec/src/lib.rs`, `crates/tak-exec/src/remote_endpoint.rs`  
Symbols: `_TOR_HIDDEN_SERVICE_CONTRACT_MARKER`, `RemoteTransportAdapter::name`, `TransportFactory::transport_name`, `#[allow(unused_imports)] use step_runner::resolve_cwd`

Why This Exists: these are leftovers from earlier enforcement tricks or abstraction experiments that were never removed.

Why It Is Unjustified: the code already admits they are unused. Keeping obvious dead weight in core files is pure noise and a reliable sign that clean-up is not happening.

Evidence:
- `takd/src/lib.rs:8-9` defines `_TOR_HIDDEN_SERVICE_CONTRACT_MARKER` behind `#[allow(dead_code)]`.
- `tak-exec/src/engine/transport.rs:2-3` and `112-115` keep the `name` path and `transport_name` path behind `#[allow(dead_code)]`.
- `tak-exec/src/lib.rs:62-63` suppresses an unused import that exists only because the include-wired module pattern obscures ownership.
- `tak-exec/src/remote_endpoint.rs:28-48` is mostly forwarding glue over `tak_core::endpoint` plus one env helper.

Simplest Refactor: delete the dead markers now, then remove the surrounding suppressions as part of the modularity cleanup.

Expected Deletion / Collapse: remove the explicit dead-code markers and the suppressions that keep them around.

## Repeated Code Consolidation Opportunities

| Opportunity | Current duplicates | Simplest consolidation |
| --- | --- | --- |
| Remote HTTP/1 exchange | `tak` CLI probe/status, `takd` Tor startup probe, `tak-exec` result fetch | One helper per crate boundary, not per call site |
| Endpoint parsing wrappers | `tak-exec/src/remote_endpoint.rs`, `takd/src/service/tor/probe.rs` | Use `tak_core::endpoint` directly |
| Tor env retry parsing | `tak-exec` transport, `tak` probe support, `takd` startup/recovery helpers | Keep one env-duration parser per crate |
| `AbortOnDrop` join-handle wrapper | `tak`, `takd`, `tak-exec` | Shared local helper or inline it into one shared request helper |
| Path containment rules | `tak-loader` and `tak` CLI override path | Move into `tak-core` |
| CPU/memory/byte formatting | `tak` remote status and `tak-exec` wait heartbeat | Keep one formatter set per presentation layer |

## Unnecessary Abstraction Inventory

| Item | Location | Why it should be cut |
| --- | --- | --- |
| Include-wired fake modules | `tak-core`, `tak-loader`, `tak-exec` crate roots | Cosmetic boundaries with hidden shared namespace |
| `RemoteTransportAdapter` trait | `crates/tak-exec/src/engine/transport.rs` | Two concrete cases do not justify a trait and factory |
| `RemoteSubmitContext` | `crates/tak-exec/src/engine/remote_models.rs` | One producer, one consumer, no domain value |
| `AttemptExecutionOutcome` | `crates/tak-exec/src/engine/attempt_execution.rs` | Temporary unpack/repack bundle |
| `RemoteHttpExchangeError` plus friends | `crates/tak-exec/src/engine/*failure*.rs` | Multiple wrappers around `kind + message` |
| `remote_endpoint` forwarding layer | `crates/tak-exec/src/remote_endpoint.rs` | Mostly renames existing core helpers |

## Suspicious Edge-Case Handling Inventory

| Case | Location | Why it looks unjustified |
| --- | --- | --- |
| Blind legacy token rewriting | `crates/tak-loader/src/loader/module_eval.rs` | Source text is rewritten without parsing, purely to preserve internal compatibility debt |
| `chunk_base64 -> chunk -> message` reconstruction ladder | `crates/takd/src/daemon/remote/route_events.rs` | Route no longer trusts its own schema and guesses |
| `RemoteTransportKind::Any` panic inside transport selection | `crates/tak-exec/src/lib.rs` | Abstraction claims to model transport selection but panics on one enum variant |
| Test-only Tor marker files in production recovery module | `crates/takd/src/service/tor/health.rs` | Test controls are mixed into production recovery code instead of being isolated |
| Handwritten Python source scanners | `crates/tak/src/docs/dsl_parse.rs`, `crates/tak/src/docs/examples_parse.rs` | Huge parsing surface built only to mine docs from internal source |

## Dead Code / Dead-Weight Inventory

| Item | Location | Why it is dead or dead-weight |
| --- | --- | --- |
| `_TOR_HIDDEN_SERVICE_CONTRACT_MARKER` | `crates/takd/src/lib.rs` | Explicit dead-code marker |
| `RemoteTransportAdapter::name` | `crates/tak-exec/src/engine/transport.rs` | Unused path kept behind suppression |
| `TransportFactory::transport_name` | `crates/tak-exec/src/engine/transport.rs` | Dead convenience function |
| `#[allow(unused_imports)] use step_runner::resolve_cwd` | `crates/tak-exec/src/lib.rs` | Include-order leak masked by suppression |
| Duplicate `chunk` text payload for streamed bytes | `crates/takd/src/daemon/remote/worker_submit_execution/output_observer.rs` | Redundant representation of the same data |
| `python3 -c DOCS_WIKI_SNIPPET` delivery path | `TASKS.py` | Inflated transport mechanism for code that should be a script file |

## Ranked Deletion Candidates

| Rank | Candidate | Why it should be deleted first |
| --- | --- | --- |
| 1 | `DOCS_WIKI_SNIPPET` in `TASKS.py` | Largest pure-noise blob with zero declarative value |
| 2 | `sanitize_canonical_v1_imports` | Blind rewrite layer masking DSL boundary debt |
| 3 | Duplicate stream chunk text payload | Hot-path duplication with no product value |
| 4 | `RemoteTransportAdapter` + `TransportFactory` | Trait/factory scaffolding around two cases |
| 5 | `_TOR_HIDDEN_SERVICE_CONTRACT_MARKER` and transport dead hooks | Explicit dead code already marked as such |

## Ranked Simplification Candidates

| Rank | Candidate | Simplest credible collapse |
| --- | --- | --- |
| 1 | `tak-exec` include-wired engine | Real submodules with explicit imports |
| 2 | Remote HTTP request paths | One helper per crate for handshake/request/body collection |
| 3 | `tak docs dump` subsystem | Replace Python scraping with structured embedded metadata |
| 4 | Loader runtime normalization | Validate defaults once and share path helpers |
| 5 | CLI execution override path | Replace helper scatter with one rewrite function |
