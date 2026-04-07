MACHINE  = "machine"
USER     = "user"
PROJECT  = "project"
WORKTREE = "worktree"

DURING   = "during"
AT_START = "at_start"

FIFO     = "fifo"
PRIORITY = "priority"

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

def module_spec(tasks, limiters=None, queues=None, exclude=None, defaults=None, project_id=None):
    return {
        "spec_version": 1,
        "project_id": project_id,
        "tasks": tasks,
        "limiters": _or_empty_list(limiters),
        "queues": _or_empty_list(queues),
        "exclude": _or_empty_list(exclude),
        "defaults": defaults if defaults is not None else {},
    }

def Local(id, max_parallel_tasks=1):
    return {
        "id": id,
        "max_parallel_tasks": max_parallel_tasks,
    }

def Remote(pool=None, required_tags=None, required_capabilities=None, transport=None, runtime=None):
    return {
        "pool": pool,
        "required_tags": _or_empty_list(required_tags),
        "required_capabilities": _or_empty_list(required_capabilities),
        "transport": transport,
        "runtime": runtime,
    }

def DirectHttps():
    return {
        "kind": "direct",
    }

def TorOnionService():
    return {
        "kind": "tor",
    }

REPO_ZIP_SNAPSHOT = "REPO_ZIP_SNAPSHOT"
OUTPUTS_AND_LOGS = "OUTPUTS_AND_LOGS"

def ContainerRuntime(image, command=None, mounts=None, env=None, resources=None):
    return {
        "kind": "containerized",
        "image": str(image),
        "command": _or_empty_list(command) if command is not None else None,
        "mounts": _or_empty_list(mounts),
        "env": _or_empty_dict(env),
        "resource_limits": resources,
    }

REASON_SIDE_EFFECTING_TASK = "SIDE_EFFECTING_TASK"
REASON_NO_REMOTE_REACHABLE = "NO_REMOTE_REACHABLE"
REASON_LOCAL_CPU_HIGH_ARM_IDLE = "LOCAL_CPU_HIGH_ARM_IDLE"
REASON_LOCAL_CPU_HIGH = "LOCAL_CPU_HIGH"
REASON_DEFAULT_LOCAL_POLICY = "DEFAULT_LOCAL_POLICY"

Reason = {
    "SIDE_EFFECTING_TASK": REASON_SIDE_EFFECTING_TASK,
    "NO_REMOTE_REACHABLE": REASON_NO_REMOTE_REACHABLE,
    "LOCAL_CPU_HIGH_ARM_IDLE": REASON_LOCAL_CPU_HIGH_ARM_IDLE,
    "LOCAL_CPU_HIGH": REASON_LOCAL_CPU_HIGH,
    "DEFAULT_LOCAL_POLICY": REASON_DEFAULT_LOCAL_POLICY,
}

def PolicyContext(task_side_effecting=False, local_cpu_percent=0.0):
    return {
        "task": {"side_effecting": bool(task_side_effecting)},
        "local": {"cpu_percent": float(local_cpu_percent)},
    }

def Decision_local(reason=REASON_DEFAULT_LOCAL_POLICY):
    return {
        "mode": "local",
        "reason": str(reason),
    }

def Decision_remote(remote, reason="DEFAULT_REMOTE_POLICY"):
    return {
        "mode": "remote",
        "remote": remote,
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

def _is_local_constructor_value(value):
    return (
        isinstance(value, dict)
        and "id" in value
        and "max_parallel_tasks" in value
        and "endpoint" not in value
    )

def _is_remote_constructor_value(value):
    return isinstance(value, dict) and "max_parallel_tasks" not in value

def LocalOnly(local):
    if not _is_local_constructor_value(local):
        raise TypeError("LocalOnly expects Local(...)")
    return {
        "kind": "local_only",
        "local": local,
    }

def RemoteOnly(remote):
    if not _is_remote_constructor_value(remote):
        raise TypeError("RemoteOnly expects Remote(...)")
    return {
        "kind": "remote_only",
        "remote": remote,
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
    reason = str(decision.get("reason", REASON_DEFAULT_LOCAL_POLICY))

    if mode == "local":
        return {
            "mode": "local",
            "reason": reason,
        }

    if mode == "remote":
        remote = decision.get("remote")
        if not _is_remote_constructor_value(remote):
            raise TypeError("Decision.remote requires Remote(...)")
        return {
            "mode": "remote",
            "reason": reason,
            "remote": remote,
        }

    raise TypeError("unsupported policy decision mode: " + str(mode))

def ByCustomPolicy(policy):
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

def path(value):
    return {
        "kind": "path",
        "value": str(value),
    }

def gitignore():
    return {
        "kind": "gitignore",
    }

def CurrentState(roots=None, ignored=None, include=None):
    return {
        "roots": _or_empty_list(roots),
        "ignored": _or_empty_list(ignored),
        "include": _or_empty_list(include),
    }

def task(name, deps=None, steps=None, needs=None, queue=None, retry=None, timeout_s=None, context=None, execution=None, tags=None, doc=None):
    return {
        "name": name,
        "deps": _normalize_deps(deps),
        "steps": _or_empty_list(steps),
        "needs": _or_empty_list(needs),
        "queue": queue,
        "retry": retry,
        "timeout_s": timeout_s,
        "context": context,
        "execution": execution,
        "tags": _or_empty_list(tags),
        "doc": doc if doc is not None else "",
    }

def cmd(*argv, cwd=None, env=None):
    return {
        "kind": "cmd",
        "argv": list(argv),
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def script(path, *argv, interpreter=None, cwd=None, env=None):
    return {
        "kind": "script",
        "path": path,
        "argv": list(argv),
        "interpreter": interpreter,
        "cwd": cwd,
        "env": _or_empty_dict(env),
    }

def need(name, slots=1, scope=PROJECT, hold=DURING):
    return {
        "limiter": {"name": name, "scope": scope},
        "slots": slots,
        "hold": hold,
    }

def queue_use(name, scope=MACHINE, slots=1, priority=0):
    return {
        "queue": {"name": name, "scope": scope},
        "slots": slots,
        "priority": priority,
    }

def resource(name, capacity, unit=None, scope=MACHINE):
    return {
        "kind": "resource",
        "name": name,
        "scope": scope,
        "capacity": capacity,
        "unit": unit,
    }

def lock(name, scope=MACHINE):
    return {
        "kind": "lock",
        "name": name,
        "scope": scope,
    }

def queue_def(name, slots, discipline=FIFO, max_pending=None, scope=MACHINE):
    return {
        "name": name,
        "scope": scope,
        "slots": slots,
        "discipline": discipline,
        "max_pending": max_pending,
    }

def rate_limit(name, burst, refill_per_second, scope=MACHINE):
    return {
        "kind": "rate_limit",
        "name": name,
        "scope": scope,
        "burst": burst,
        "refill_per_second": refill_per_second,
    }

def process_cap(name, max_running, match=None, scope=MACHINE):
    return {
        "kind": "process_cap",
        "name": name,
        "scope": scope,
        "max_running": max_running,
        "match": match,
    }

def retry(attempts=1, on_exit=None, backoff=None):
    return {
        "attempts": attempts,
        "on_exit": _or_empty_list(on_exit),
        "backoff": backoff if backoff is not None else fixed(0),
    }

def fixed(seconds):
    return {
        "kind": "fixed",
        "seconds": seconds,
    }

def exp_jitter(min_s=1, max_s=60, jitter="full"):
    return {
        "kind": "exp_jitter",
        "min_s": min_s,
        "max_s": max_s,
        "jitter": jitter,
    }
