_Scope_Machine  = "machine"
_Scope_User     = "user"
_Scope_Project  = "project"
_Scope_Worktree = "worktree"

_Hold_During   = "during"
_Hold_AtStart = "at_start"

_QueueDiscipline_Fifo     = "fifo"
_QueueDiscipline_Priority = "priority"

_SessionLifetime_PerRun = "per_run"

def _or_empty_list(value):
    return value if value is not None else []

def _or_empty_dict(value):
    return value if value is not None else {}

def _dep_to_label(value):
    if isinstance(value, str):
        return value
    if isinstance(value, dict):
        name = value.get("name")
        if isinstance(name, str):
            if name.startswith("//") or name.startswith(":"):
                return name
            return ":" + name
    raise TypeError("dependency must be a label string or a task object")

def _normalize_deps(value):
    if value is None:
        return []
    if isinstance(value, list):
        return [_dep_to_label(item) for item in value]
    return [_dep_to_label(value)]

def module_spec(tasks, limiters=None, queues=None, exclude=None, includes=None, defaults=None, project_id=None, sessions=None, execution_policies=None):
    """Declare the module boundary that Tak loads from one TASKS.py file."""
    return {
        "spec_version": 1,
        "project_id": project_id,
        "tasks": tasks,
        "sessions": _or_empty_list(sessions),
        "limiters": _or_empty_list(limiters),
        "queues": _or_empty_list(queues),
        "execution_policies": _or_empty_list(execution_policies),
        "exclude": _or_empty_list(exclude),
        "includes": _or_empty_list(includes),
        "defaults": defaults if defaults is not None else {},
    }

def _is_host_runtime(value):
    return isinstance(value, dict) and value.get("kind") == "host"

def _normalize_local_runtime(runtime):
    if runtime is None or _is_host_runtime(runtime):
        return None
    return runtime

def _local_spec(runtime=None):
    return {
        "id": "local",
        "max_parallel_tasks": 1,
        "runtime": _normalize_local_runtime(runtime),
    }

def _remote_selection(selection=None):
    return selection if selection is not None else {"kind": "sequential"}

def _remote_spec(pool=None, required_tags=None, required_capabilities=None, transport=None, runtime=None, selection=None):
    if _is_host_runtime(runtime):
        raise TypeError("Runtime.Host() is only valid for Execution.Local")
    return {
        "pool": pool,
        "required_tags": _or_empty_list(required_tags),
        "required_capabilities": _or_empty_list(required_capabilities),
        "transport": transport,
        "runtime": runtime,
        "selection": _remote_selection(selection),
    }

def RemoteSelection_Sequential():
    """Try matching remotes in inventory order."""
    return {
        "kind": "sequential",
    }

def RemoteSelection_Shuffle():
    """Spread attempts across matching remotes with deterministic per-attempt ordering."""
    return {
        "kind": "shuffle",
    }

def Transport_DirectHttps():
    """Force direct HTTPS transport for a remote target."""
    return {
        "kind": "direct",
    }

def Transport_Any():
    """Allow Tak to choose direct or Tor transport from the available remote endpoint."""
    return {
        "kind": "any",
    }

def Transport_TorOnionService():
    """Force Tor onion-service transport for a remote target."""
    return {
        "kind": "tor",
    }

def Runtime_Host():
    """Run local work directly on the host without a container."""
    return {
        "kind": "host",
    }

def Runtime_Image(image, command=None, mounts=None, env=None, resources=None):
    """Run work inside a prebuilt container image."""
    return {
        "kind": "containerized",
        "image": str(image),
        "dockerfile": None,
        "build_context": None,
        "command": _or_empty_list(command) if command is not None else None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": resources,
    }

def Runtime_Dockerfile(dockerfile, build_context=None, command=None, mounts=None, env=None, resources=None):
    """Build a container runtime from a Dockerfile in the workspace."""
    return {
        "kind": "containerized",
        "image": None,
        "dockerfile": dockerfile if isinstance(dockerfile, dict) else path(dockerfile),
        "build_context": (
            build_context if isinstance(build_context, dict) or build_context is None else path(build_context)
        ),
        "command": _or_empty_list(command) if command is not None else None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": resources,
    }

def PolicyContext(task_side_effecting=False, local_cpu_percent=0.0):
    """Provide the runtime facts exposed to a custom placement policy."""
    return {
        "task": {"side_effecting": bool(task_side_effecting)},
        "local": {"cpu_percent": float(local_cpu_percent)},
    }

def _is_local_constructor_value(value):
    return (
        isinstance(value, dict)
        and "id" in value
        and "max_parallel_tasks" in value
        and "endpoint" not in value
    )

def _is_remote_constructor_value(value):
    return isinstance(value, dict) and "max_parallel_tasks" not in value

def Decision_local(reason="DEFAULT_LOCAL_POLICY", runtime=None):
    """Return an explicit local placement decision from a custom policy."""
    decision = {
        "mode": "local",
        "reason": str(reason),
    }
    normalized_runtime = _normalize_local_runtime(runtime)
    if normalized_runtime is not None:
        decision["local"] = _local_spec(runtime=normalized_runtime)
    return decision

def Decision_remote(reason="DEFAULT_REMOTE_POLICY", pool=None, required_tags=None, required_capabilities=None, transport=None, runtime=None):
    """Return an explicit remote placement decision from a custom policy."""
    return {
        "mode": "remote",
        "remote": _remote_spec(
            pool=pool,
            required_tags=required_tags,
            required_capabilities=required_capabilities,
            transport=transport,
            runtime=runtime,
        ),
        "reason": str(reason),
    }

def _unsupported_policy_builder_api(name):
    raise TypeError(
        "unsupported policy builder API: "
        + str(name)
        + " (use Decision.local/Decision.remote)"
    )

def Decision_start(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.start")

def Decision_prefer_local(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.prefer_local")

def Decision_prefer_remote(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.prefer_remote")

def Decision_resolve(*args, **kwargs):
    _unsupported_policy_builder_api("Decision.resolve")

POLICY_CONTEXT = PolicyContext()

def Execution_Local(runtime=None):
    """Force a task to run locally, on the host by default or inside the supplied runtime."""
    return {
        "kind": "local_only",
        "local": _local_spec(runtime=runtime),
    }

def Execution_Remote(pool=None, required_tags=None, required_capabilities=None, transport=None, runtime=None, selection=None):
    """Force a task to run remotely with the supplied target filters and runtime."""
    return {
        "kind": "remote_only",
        "remote": _remote_spec(
            pool=pool,
            required_tags=required_tags,
            required_capabilities=required_capabilities,
            transport=transport,
            runtime=runtime,
            selection=selection,
        ),
    }

def _compile_policy_decision(policy, context):
    decision = policy(context)
    if not isinstance(decision, dict):
        raise TypeError("policy function must return Decision.local/remote")

    scoring_fields = []
    if "score" in decision:
        scoring_fields.append("score")
    if "weight" in decision:
        scoring_fields.append("weight")
    if len(scoring_fields) > 0:
        raise TypeError(
            "unsupported policy scoring fields: " + ", ".join(scoring_fields)
        )

    mode = decision.get("mode")
    reason = str(decision.get("reason", "DEFAULT_LOCAL_POLICY"))

    if mode == "local":
        local = decision.get("local")
        if local is not None and not _is_local_constructor_value(local):
            raise TypeError("Decision.local requires Runtime.Host/Image/Dockerfile")
        compiled = {
            "mode": "local",
            "reason": reason,
        }
        if local is not None:
            compiled["local"] = local
        return compiled

    if mode == "remote":
        remote = decision.get("remote")
        if not _is_remote_constructor_value(remote):
            raise TypeError("Decision.remote requires remote execution arguments")
        return {
            "mode": "remote",
            "reason": reason,
            "remote": remote,
        }

    raise TypeError("unsupported policy decision mode: " + str(mode))

def Execution_Policy(policy):
    """Resolve task placement from a named or inline custom policy."""
    if not isinstance(POLICY_CONTEXT, dict):
        raise TypeError("POLICY_CONTEXT must be PolicyContext(...)")

    if not isinstance(policy, str):
        decision = _compile_policy_decision(policy, POLICY_CONTEXT)
        return {
            "kind": "by_custom_policy",
            "policy_name": str(policy),
            "decision": decision,
        }
    return {
        "kind": "by_custom_policy",
        "policy_name": str(policy),
    }

def SessionReuse_Workspace():
    """Reuse one per-run session workspace across every task in the session."""
    return {
        "kind": "share_workspace",
    }

def SessionReuse_Paths(paths):
    """Persist only the selected paths or globs between tasks in one session."""
    return {
        "kind": "share_paths",
        "paths": _or_empty_list(paths),
    }

def execution_policy(name, placements, doc=None):
    """Declare an ordered named execution policy for local and remote placements."""
    return {
        "name": str(name),
        "placements": _or_empty_list(placements),
        "doc": doc if doc is not None else "",
    }

def session(name, execution=None, reuse=None, lifetime=_SessionLifetime_PerRun, context=None, execution_policy=None):
    """Declare a named per-run execution session for containerized task chains."""
    if execution is not None and execution_policy is not None:
        raise TypeError("session `" + str(name) + "` cannot set both execution and execution_policy")
    return {
        "name": str(name),
        "execution": execution,
        "execution_policy": execution_policy,
        "reuse": reuse,
        "lifetime": lifetime,
        "context": context,
    }

def Execution_Session(name, cascade=False):
    """Run a task in a named session, optionally cascading it to dependencies."""
    return {
        "kind": "use_session",
        "name": str(name),
        "cascade": bool(cascade),
    }

def path(value):
    """Reference one workspace path in Tak inputs or outputs."""
    return {
        "kind": "path",
        "value": str(value),
    }

def glob(value):
    """Reference a glob pattern in Tak inputs or outputs."""
    return {
        "kind": "glob",
        "value": str(value),
    }

def gitignore():
    """Reuse the repo's gitignore rules as a CurrentState ignore source."""
    return {
        "kind": "gitignore",
    }

def CurrentState(roots=None, ignored=None, include=None):
    """Capture the current workspace contents as an execution input snapshot."""
    return {
        "roots": _or_empty_list(roots),
        "ignored": _or_empty_list(ignored),
        "include": _or_empty_list(include),
    }

def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, context=None, outputs=None, execution=None, execution_policy=None, tags=None, doc=None):
    """Declare one task, including its steps, dependencies, execution policy, and outputs."""
    if execution is not None and execution_policy is not None:
        raise TypeError("task `" + str(name) + "` cannot set both execution and execution_policy")
    return {
        "name": name,
        "deps": _normalize_deps(deps),
        "steps": _or_empty_list(steps),
        "needs": _or_empty_list(needs),
        "queue": queue,
        "retry": retry,
        "timeout_s": timeout_s,
        "context": context,
        "outputs": _or_empty_list(outputs),
        "execution": execution,
        "execution_policy": execution_policy,
        "tags": _or_empty_list(tags),
        "doc": doc if doc is not None else "",
    }

def cmd(*argv, cwd=None, env=None):
    """Run one command step with optional cwd and environment overrides."""
    return {
        "kind": "cmd",
        "argv": list(argv),
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def script(path, *argv, interpreter=None, cwd=None, env=None):
    """Run one checked-in script step with optional interpreter, cwd, and environment overrides."""
    return {
        "kind": "script",
        "path": path,
        "argv": list(argv),
        "interpreter": interpreter,
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def need(name, slots=1, scope=_Scope_Project, hold=_Hold_During):
    """Acquire slots from a limiter while a task runs or starts."""
    return {
        "limiter": {"name": name, "scope": scope},
        "slots": slots,
        "hold": hold,
    }

def queue_use(name, scope=_Scope_Machine, slots=1, priority=0):
    """Join a named queue before the task starts."""
    return {
        "queue": {"name": name, "scope": scope},
        "slots": slots,
        "priority": priority,
    }

def resource(name, capacity, unit=None, scope=_Scope_Machine):
    """Define a capacity-based limiter such as CPU or RAM slots."""
    return {
        "kind": "resource",
        "name": name,
        "scope": scope,
        "capacity": capacity,
        "unit": unit,
    }

def lock(name, scope=_Scope_Machine):
    """Define an exclusive limiter with one available slot."""
    return {
        "kind": "lock",
        "name": name,
        "scope": scope,
    }

def queue_def(name, slots, discipline=_QueueDiscipline_Fifo, max_pending=None, scope=_Scope_Machine):
    """Define a queue and its scheduling discipline."""
    return {
        "name": name,
        "scope": scope,
        "slots": slots,
        "discipline": discipline,
        "max_pending": max_pending,
    }

def rate_limit(name, burst, refill_per_second, scope=_Scope_Machine):
    """Define a token-bucket limiter."""
    return {
        "kind": "rate_limit",
        "name": name,
        "scope": scope,
        "burst": burst,
        "refill_per_second": refill_per_second,
    }

def process_cap(name, max_running, match=None, scope=_Scope_Machine):
    """Define a limiter that matches and caps external processes."""
    return {
        "kind": "process_cap",
        "name": name,
        "scope": scope,
        "max_running": max_running,
        "match": match,
    }

def retry(attempts=1, on_exit=None, backoff=None):
    """Configure retry attempts, exit-code matching, and backoff."""
    return {
        "attempts": attempts,
        "on_exit": _or_empty_list(on_exit),
        "backoff": backoff if backoff is not None else fixed(0),
    }

def fixed(seconds):
    """Use a fixed retry backoff duration."""
    return {
        "kind": "fixed",
        "seconds": seconds,
    }

def exp_jitter(min_s=1, max_s=60, jitter="full"):
    """Use exponential backoff with jitter."""
    return {
        "kind": "exp_jitter",
        "min_s": min_s,
        "max_s": max_s,
        "jitter": jitter,
    }
