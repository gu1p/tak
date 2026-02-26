# AGENTS (Rust)

This `AGENTS.md` applies to this Rust package/folder and complements the root rules.
If there is any conflict, follow the repo-root `AGENTS.md` first, then this one.

## North Star

Write Rust that is:
- **easy to read**, easy to change, hard to misuse
- organized around **Clean Architecture**
- composed of **small units** that do one thing
- expressive: code should **tell a story**

The best code here is **self-explanatory** without requiring comments to understand intent.

---

## Architecture (Clean Architecture, Rust-style)

Structure code by **direction of dependency** (not by framework/IO details):

1. **Domain** (pure business rules)
   - Entities, value objects, domain services
   - No IO, no DB, no web, no time, no randomness
2. **Application / Use Cases**
   - Orchestrates domain behavior
   - Defines ports (traits) needed from the outside world
3. **Interface Adapters**
   - Implements ports: repositories, gateways, presenters, mappers
   - Translates between domain types and external representations
4. **Infrastructure**
   - DB clients, HTTP clients, filesystem, CLI, framework glue

**Rule:** Dependencies point inward.  
Infrastructure depends on application/domain, never the other way around.

---

## “OO” in Rust (Traits + Composition)

We do OO as:
- **behavior via traits**
- **composition over inheritance**
- **data + invariants** in domain types
- **dynamic dispatch only when it buys something** (otherwise generics)

### Ports & Adapters
- Define interfaces as **traits** in the inner layers (application/domain).
- Implement those traits in outer layers (adapters/infrastructure).
- Traits should read like a **story of the system** (clear verbs, clear nouns).

---

## Clean Code Rules (Non-negotiable)

### Small, single-purpose functions
- Functions/methods should be **very small** and do **one thing**.
- If you need “and then”, “also”, or “but” to explain a function, split it.
- Avoid deep nesting. Prefer early returns and small helpers.

### Naming is design
- Names must encode intent and make the flow readable.
- Prefer domain language: `Invoice`, `Reservation`, `Policy`, `AuthorizePayment`.
- Avoid generic names: `data`, `handle`, `process`, `do_stuff`, `mgr`.

### Abstractions must tell a story
- Traits and modules should describe *why* the code exists, not *how* it’s done.
- Don’t abstract “because we can”; abstract because it clarifies intent or isolates volatility.

### Comments are not a crutch
- Prefer making code readable over explaining it with comments.
- Comments are allowed for:
  - non-obvious invariants
  - tricky performance tradeoffs (with reasoning)
  - safety notes around `unsafe` (required)

---

## Error Handling & Contracts

- Use `Result` and explicit error types; avoid `unwrap()`/`expect()` in production code.
- Errors should be meaningful at the boundary they are created.
- Validate invariants at the edges:
  - parsing/decoding at the boundary
  - domain invariants inside domain constructors/factories

---

## Boundaries, IO, and Purity

- Keep IO at the edges. Most code should be testable without IO.
- Prefer passing explicit dependencies (ports) into constructors/functions.
- Avoid global state and hidden side effects.

---

## Testing Expectations

Follow repo TDD rules. In Rust terms:
- Domain and use cases should be **easy to unit test** (pure inputs/outputs).
- Use-case tests should assert behavior via ports (traits) using fakes/mocks.
- Adapter/infrastructure tests verify integration with external systems.

If a bug is found:
1. Write a failing test reproducing it.
2. Fix with minimal change.
3. Add regression coverage at the right layer.

## Doctest Contract (Mandatory)

- Every documented function in `crates/*/src/**/*.rs` must include at least one fenced Rust example.
- Allowed fenced languages:
  - ```` ```rust ````
  - ```` ```no_run ````
  - ```` ```compile_fail ````
- Forbidden fenced language:
  - ```` ```ignore ````.
- Any `no_run` or `compile_fail` block must include `Reason:` in the same fenced block.
- Keep examples deterministic and minimal. Prefer hidden setup lines (`# ...`) to reduce noise.
- `fn main` wrappers are excluded from this strict requirement.
- CI/local enforcement:
  - `cargo test --workspace --doc`
  - `cargo test -p tak --test doctest_contract`

---

## Rust Style & Idioms (Readability First)

- Prefer simple control flow over cleverness.
- Keep lifetimes and generics as simple as possible; introduce complexity only when necessary.
- Use `clippy` guidance, but choose clarity over pedantic micro-optimizations.
- `unsafe` is exceptional:
  - isolate it
  - document invariants
  - wrap it behind a safe API
  - test it directly

---

## PR / Change Hygiene

- Changes are small and cohesive.
- No drive-by refactors mixed with behavior changes.
- Public APIs should remain stable unless there’s a clear migration plan.

---

## Quick Heuristics

If you’re unsure:
- Split functions until each reads like a sentence.
- Move decisions inward (domain/use case) and IO outward (adapters).
- Name traits like capabilities: `UserRepository`, `Clock`, `PaymentGateway`.
- Make the “happy path” obvious and linear; push edge cases into small helpers.
