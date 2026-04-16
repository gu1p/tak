# AGENTS

This document defines **non-negotiable** development rules for humans and agents contributing to this repo.
For Rust-specific coding style and guidelines, also read RUST_GUIDELINES.md

## Core Principles

- We practice **strict Test-Driven Development (TDD)**.
- **No production code changes without a test first.**  
  - New feature → add tests that specify the behavior.
  - Bug fix → add a failing test that reproduces the bug, then fix.
- Changes must be **small, reviewable, and reversible**.
- Always keep the product UX stable: the UI must not “silently degrade” through refactors.

## Agent Knowledge Extraction

- Before authoring or revising `TASKS.py`, run `tak docs dump`.
- Treat that bundle as the canonical extraction path for:
  - Tak capabilities
  - the shipped `TASKS.py` DSL surface
  - example selection for similar project shapes
- Prefer adapting the closest shipped example from the bundle instead of inventing a new pattern first.

## Required Workflow (Red → Green → Refactor)

Work proceeds in this exact order:

1. **BDD / Behavioral tests first**  
   - Covers functional behavior and **UI contract/presence** (see below).
2. **Unit tests second**
3. **Integration tests third**
4. **Implementation code last**
5. **Refactor/adjust** (only after tests are green)

> If you find yourself editing implementation code before writing a failing test, stop and fix the process.

## UX & UI Contract Testing (Project-Critical)

This project is **UX-heavy**. Beyond functional correctness, we require strong guarantees that core UI elements remain present and accessible across refactors.

UI contract tests must ensure (as applicable):

- Key components **exist**: menus, buttons, filters, columns, panels, dialogs, toolbars, etc.
- States are covered:
  - **Empty states**
  - **Loading states**
  - **Error states**
- Elements remain **reachable and usable** (not just rendered).
- Assertions are **stable**:
  - Prefer semantic queries / roles / labels (accessibility-first).
  - Avoid brittle selectors (e.g., deep class chains) unless there’s no alternative.
- If a UI element is intentionally removed or changed, tests must be updated **with an explicit reason** in the PR description.

## Tests Are the Spec

- Tests define the contract of the system.
- Implement the **minimum code necessary** to make the new test pass.
- Refactor only when tests are green and coverage is representative.

## Mandatory Checks

- Run `tak run //:check` for **every single change** (features, fixes, refactors, docs that affect tooling).
- Use `tak exec -- ...` for isolated test or tool-native loops after adding a test and before the final full gate.
- Do not merge code that fails `tak run //:check`.
- Do not report work as complete until the most recent `tak run //:check` run exits successfully in the current branch state.
- If `tak run //:check` fails, report the failing test/spec names and keep the task in-progress.

## Definition of Done (DoD)

A change is “done” only when:

- Relevant BDD/UI contract tests exist (when UX is involved).
- Unit/integration tests are added/updated as needed.
- `tak run //:check` passes locally.
- The completion report includes the executed validation command(s) and pass/fail result.
- The change is minimal, readable, and avoids unrelated refactors.

## Common Scenarios

### Adding a Feature
1. Write BDD tests describing the behavior and UI contract.
2. Add unit tests for core logic.
3. Add integration tests if cross-module behavior exists.
4. If it a flow that an user or other stakeholder will perform, add a E2E test mimicking their use -- as real as we can.
5. Use `tak exec -- <tool> ...` for narrow feedback loops such as one new test or one package-specific test run.
6. Run `tak run //:check`.
7. Implement minimal code to pass.
8. Refactor safely.

### Fixing a Bug
1. Add a test that fails on the current code and reproduces the bug.
2. Implement the smallest fix.
3. Add/adjust regression coverage (unit/integration as appropriate).
4. Use `tak exec -- <tool> ...` for focused validation when needed.
5. Run `tak run //:check`.

---

If you are unsure which test type to start with, default to **BDD/UI contract tests** for anything user-visible.
