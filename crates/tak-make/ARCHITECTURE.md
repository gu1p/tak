# tak-make Architecture

## Purpose

`tak-make` is the Makefile-facing application boundary for `tak make <goal>`. It resolves a small,
explicit annotation language and asks an injected executor to run either one opaque Make invocation
or an explicitly annotated graph of phony Make targets. Make remains the build engine for every
individual node.

## Dependency Direction

The crate separates pure Make-domain decisions from effects:

1. Domain values describe placement, container source, Makefile source, and execution outcome.
2. The application use case reads a Makefile, resolves one goal, and submits one invocation or a
   parallel execution plan.
3. Ports (`MakefileReader` and `GoalExecutor`) own filesystem and process/runtime effects.
4. The filesystem adapter implements default Makefile lookup.
5. The `tak` CLI supplies the outer adapter that lowers the plan into a protocol-v2 run submission
   for local `takd`.

Tests inject an in-memory reader and recording executor, so annotation behavior does not require a
filesystem, Make binary, container engine, or remote agent.

## Supported Source Contract

Default lookup follows GNU Make precedence: `GNUmakefile`, `makefile`, then `Makefile`. The parser
accepts literal single-target headers such as `test: build`. A contiguous block immediately above
that header may contain:

```make
# tak: execution=remote
# tak: container-image=alpine:3.20
test: build
	./scripts/test.sh
```

File-wide defaults use common execution/container keys and `parallel-output` with a `default.`
prefix and may appear in top-level Makefile comments. `default.parallel` is rejected because a
file-wide graph would have no owning aggregate:

```make
# tak: default.execution=remote
# tak: default.container-image=alpine:3.20
```

Supported keys are:

- `execution=local|remote`
- `container-image=<reference>`
- `container-dockerfile=<workspace path>`
- `container-build-context=<workspace path>`
- `parallel=<direct-goal,direct-goal,...>`
- `parallel-output=live|grouped`

A blank line or ordinary comment breaks association. Image and Dockerfile sources are mutually
exclusive within one scope. Resolution applies global defaults first, then compatible goal fields;
a goal build context may inherit the default Dockerfile, while choosing an image or Dockerfile
replaces the mutually exclusive source. The outer CLI applies command-line overrides last. An
inherited container can be ignored for one invocation with `--local-no-container`. Unsupported
annotated declarations fail at the parser boundary.

## Deliberate Limits

The parser does not evaluate Make. It does not resolve included files, variables in target names,
generated rules, multi-target rules, pattern rules, static-pattern rules, or double-colon rules.
Target-specific variable assignments are also outside the annotation grammar. This narrow surface
is deliberate: silently attaching remote/container policy to the wrong command would be worse than
rejecting syntax Tak cannot identify safely.

Without `parallel`, the executor receives `argv = ["make", goal]` and preserves the original opaque
behavior. With `parallel`, the parser emits a recursive DAG in stable left-to-right order. Leaf nodes
run `make child`; join nodes run `make --assume-old=child ... aggregate` after their promoted children
finish. Every promoted target must be literal, direct, unique, and phony. Dynamic prerequisites,
cycles, and conflicting inherited settings fail before execution. The CLI lowers the plan into one
hard-affined `SharedWorkspace`, so a dependent promoted goal observes files written by successful
prerequisites. The selected root task declares the shared workspace with a `**` output. After the
daemon commits that output, the client safely materializes it with the standard checkout-conflict
preflight; the complete artifact remains retrievable if local files changed.

The Make adapter never launches the resulting commands itself. `tak make` requires local `takd`
for local and remote placement and has no client executor fallback.
