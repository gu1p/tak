# tak-make Architecture

## Purpose

`tak-make` is the Makefile-facing application boundary for `tak make <goal>`. It resolves a small,
explicit annotation language and asks an injected executor to run one Make invocation. It never
interprets Make prerequisites or turns them into Tak tasks; Make remains the build engine.

## Dependency Direction

The crate separates pure Make-domain decisions from effects:

1. Domain values describe placement, container source, Makefile source, and execution outcome.
2. The application use case reads a Makefile, resolves one goal, and submits an execution request.
3. Ports (`MakefileReader` and `GoalExecutor`) own filesystem and process/runtime effects.
4. The filesystem adapter implements default Makefile lookup.
5. The `tak` CLI supplies the outer `GoalExecutor` adapter that lowers the request into `tak-exec`.

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

Supported keys are:

- `execution=local|remote`
- `container-image=<reference>`
- `container-dockerfile=<workspace path>`
- `container-build-context=<workspace path>`

A blank line or ordinary comment breaks association. Image and Dockerfile sources are mutually
exclusive. Unsupported annotated declarations fail at the parser boundary.

## Deliberate Limits

The parser does not evaluate Make. It does not resolve included files, variables in target names,
generated rules, multi-target rules, pattern rules, static-pattern rules, or double-colon rules.
Target-specific variable assignments are also outside the annotation grammar. This narrow surface
is deliberate: silently attaching remote/container policy to the wrong command would be worse than
rejecting syntax Tak cannot identify safely.

The executor receives `argv = ["make", goal]` and the selected default Makefile path for diagnostics.
It runs the bare Make command so Make retains normal behavior. The first version declares no Tak
output paths; remote stdout/stderr and exit status return, but remote-generated files do not.
