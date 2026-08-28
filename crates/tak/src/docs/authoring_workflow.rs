//! Start from the closest example and keep intent next to the source with `task(doc=...)`,
//! crate docs, and command doc comments. Choose the authoring path for the entry point you need:
//!
//! - Annotate an existing Makefile when GNU Make already owns the dependency graph and recipes.
//! - Create `TASKS.py` when Tak should own the graph, declared inputs and outputs, retries,
//!   coordination, or per-task execution policy.
//!
//! The paths are independent: `tak make` does not load `TASKS.py`, and Tak's graph commands do
//! not infer tasks from a Makefile. Add only the execution, retry, coordination, and remote
//! constructs the project actually needs.
//!
//! ### Annotate an existing Makefile
//!
//! Do not create a `TASKS.py` just to wrap an existing Make goal. Without annotations,
//! `tak make <goal>` runs one ordinary `make <goal>` invocation. Put goal annotations in one
//! contiguous `# tak: key=value` comment block immediately above a literal, single-target rule.
//! A blank line or ordinary comment breaks that association. File-wide defaults may appear
//! separately and use the `default.` prefix.
//!
//! ```make
//! # tak: default.execution=remote
//! # tak: default.container-image=ghcr.io/acme/build:latest
//!
//! .PHONY: check lint test
//!
//! # tak: parallel=lint,test
//! # tak: parallel-output=grouped
//! check: lint test
//! 	@echo checks passed
//!
//! lint:
//! 	@echo lint
//!
//! test:
//! 	@echo test
//! ```
//!
//! To configure only one goal, attach the container and placement to that goal instead of using
//! file-wide defaults:
//!
//! ```make
//! # tak: execution=remote
//! # tak: container-dockerfile=docker/test.Dockerfile
//! # tak: container-build-context=.
//! integration: build
//! 	./scripts/integration.sh
//! ```
//!
//! Supported goal annotation keys are `execution=local|remote`,
//! `container-image=<reference>`, `container-dockerfile=<path>`,
//! `container-build-context=<path>`, `parallel=<goal,goal,...>`, and
//! `parallel-output=live|grouped`. All except `parallel` may use the `default.` prefix.
//! Goal annotations override defaults; CLI flags override all authored annotations.
//! `container-image` and `container-dockerfile` are mutually exclusive, and
//! `container-build-context` requires `container-dockerfile`.
//!
//! A `parallel` annotation needs at least two unique, direct, literal prerequisites. The group and
//! every promoted member must be declared in `.PHONY`. Tak intentionally rejects annotations on
//! generated, pattern, multi-target, double-colon, or continued rules. Remote Make execution does
//! not materialize generated files back into the local workspace, so promoted remote goals must
//! not consume one another's generated files. Run the chosen goal, for example `tak make check`,
//! to validate the annotations and execute it.
//!
//! ### Create a TASKS.py workspace
//!
//! `TASKS.py` is evaluated with Tak's DSL already in scope; do not add imports for `task`, `cmd`,
//! or `module_spec`. Define task values, return them from one `module_spec(...)`, and leave that
//! module spec as the file's final expression.
//!
//! ```python
//! build = task(
//!     "build",
//!     doc="Build the project.",
//!     outputs=[path("out/build.txt")],
//!     steps=[cmd("sh", "-c", "mkdir -p out && echo built > out/build.txt")],
//! )
//!
//! check = task(
//!     "check",
//!     doc="Check the built project.",
//!     deps=[":build"],
//!     steps=[cmd("sh", "-c", "test -f out/build.txt")],
//! )
//!
//! SPEC = module_spec(
//!     project_id="acme_project",
//!     tasks=[build, check],
//! )
//! SPEC
//! ```
//!
//! A label beginning with `:` refers to a task in the same module. From the workspace root,
//! `//:check` is the absolute label for the root task named `check`.
//! Inspect the graph without executing it:
//!
//! ```text
//! tak list
//! tak explain //:check
//! tak graph //:check --format dot
//! ```
//!
//! Then execute the task:
//!
//! ```text
//! tak run //:check
//! ```
//!
//! For a larger project, select the closest entry in **Project Patterns** and **Example Chooser**
//! below, copy its complete embedded source files, then change only what the project needs.

#![allow(clippy::tabs_in_doc_comments)] // Make recipes require literal tab characters.
